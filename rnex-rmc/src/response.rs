// i seriously dont know why the compiler is complaining about unused parentheses in the repr
// attributes but this gets it to not complain anymore
#![allow(unused_parens)]

use crate::qresult::ERROR_MASK;
use crate::serialization::Error;
use bytemuck::bytes_of;
use rnex_util::SendingBufferConnection;
use std::io;
use std::io::{Read, Seek, Write};
use tracing::{error, warn};
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};

#[derive(Debug, Clone)]
pub enum RMCResponseResult {
    Success {
        call_id: u32,
        method_id: u32,
        data: Vec<u8>,
    },
    Error {
        error_code: ErrorCode,
        call_id: u32,
    },
}

#[derive(Debug, Clone)]
pub struct RMCResponse {
    pub protocol_id: u8,
    pub response_result: RMCResponseResult,
}

impl RMCResponse {
    pub fn new(stream: &mut (impl Seek + Read)) -> io::Result<Self> {
        // ignore the size for now this will only be used for checking
        let size: u32 = stream.read_struct(IS_BIG_ENDIAN)?;

        let protocol_id: u8 = stream.read_struct(IS_BIG_ENDIAN)?;

        /*let protocol_id: u16 = match protocol_id{
            0x7F => {
                stream.read_struct(IS_BIG_ENDIAN)?
            },
            _ => protocol_id as u16
        };*/

        let is_success: u8 = stream.read_struct(IS_BIG_ENDIAN)?;

        let response_result = if is_success == 0x01 {
            let call_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let method_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let method_id = method_id & (!0x8000);

            let mut data: Vec<u8> = vec![0u8; (size - 2 - 4 - 4) as _];

            stream.read(&mut data)?;

            RMCResponseResult::Success {
                call_id,
                method_id,
                data,
            }
        } else {
            let error_code: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let error_code = error_code & (!0x8000_0000);
            let call_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;

            RMCResponseResult::Error {
                error_code: {
                    match ErrorCode::try_from(error_code) {
                        Ok(v) => v,
                        Err(_) => {
                            error!("invalid error code {:#010x}", error_code);
                            ErrorCode::Core_Exception
                        }
                    }
                },
                call_id,
            }
        };

        Ok(Self {
            protocol_id,
            response_result,
        })
    }

    pub fn get_call_id(&self) -> u32 {
        match &self.response_result {
            RMCResponseResult::Success { call_id, .. } => *call_id,
            RMCResponseResult::Error { call_id, .. } => *call_id,
        }
    }

    pub fn to_data(self) -> Vec<u8> {
        generate_response(self.protocol_id, self.response_result)
            .expect("failed to generate response")
    }
}

pub fn generate_response(protocol_id: u8, response: RMCResponseResult) -> io::Result<Vec<u8>> {
    let size = 1
        + 1
        + match &response {
            RMCResponseResult::Success { data, .. } => 4 + 4 + data.len(),
            RMCResponseResult::Error { .. } => 4 + 4,
        };

    let mut data_out = Vec::with_capacity(size + 4);

    let u32_size: u32 = size as _;

    data_out.write_all(bytes_of(&u32_size))?;
    data_out.push(protocol_id);

    match response {
        RMCResponseResult::Success {
            call_id,
            method_id,
            data,
        } => {
            data_out.push(1);
            data_out.write_all(bytes_of(&call_id))?;
            let ored_method_id = method_id | 0x8000;
            data_out.write_all(bytes_of(&ored_method_id))?;
            data_out.write_all(&data)?;
        }
        RMCResponseResult::Error {
            call_id,
            error_code,
        } => {
            data_out.push(0);
            let error_code_val: u32 = error_code.into();
            let error_code_val = error_code_val | ERROR_MASK;
            data_out.write_all(bytes_of(&error_code_val))?;
            data_out.write_all(bytes_of(&call_id))?;
        }
    }

    assert_eq!(data_out.len(), size + 4);

    Ok(data_out)
}

pub async fn send_result(
    connection: &SendingBufferConnection,
    result: Result<Vec<u8>, ErrorCode>,
    protocol_id: u8,
    method_id: u32,
    call_id: u32,
) {
    let response_result = match result {
        Ok(v) => RMCResponseResult::Success {
            call_id,
            method_id,
            data: v,
        },
        Err(e) => {
            warn!("error occurred during call: {:?}", e);
            RMCResponseResult::Error {
                call_id,
                error_code: e.into(),
            }
        }
    };

    let response = RMCResponse {
        response_result,
        protocol_id,
    };

    send_response(connection, response).await
}

pub async fn send_response(connection: &SendingBufferConnection, rmcresponse: RMCResponse) {
    connection.send(rmcresponse.to_data()).await;
}

//taken from kinnays error list directly
#[allow(nonstandard_style)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    Core_Unknown = 0x0001_0001,
    Core_NotImplemented = 0x0001_0002,
    Core_InvalidPointer = 0x0001_0003,
    Core_OperationAborted = 0x0001_0004,
    Core_Exception = 0x0001_0005,
    Core_AccessDenied = 0x0001_0006,
    Core_InvalidHandle = 0x0001_0007,
    Core_InvalidIndex = 0x0001_0008,
    Core_OutOfMemory = 0x0001_0009,
    Core_InvalidArgument = 0x0001_000A,
    Core_Timeout = 0x0001_000B,
    Core_InitializationFailure = 0x0001_000C,
    Core_CallInitiationFailure = 0x0001_000D,
    Core_RegistrationError = 0x0001_000E,
    Core_BufferOverflow = 0x0001_000F,
    Core_InvalidLockState = 0x0001_0010,
    Core_InvalidSequence = 0x0001_0011,
    Core_SystemError = 0x0001_0012,
    Core_Cancelled = 0x0001_0013,
    DDL_InvalidSignature = 0x0002_0001,
    DDL_IncorrectVersion = 0x0002_0002,
    RendezVous_ConnectionFailure = 0x0003_0001,
    RendezVous_NotAuthenticated = 0x0003_0002,
    RendezVous_InvalidUsername = 0x0003_0064,
    RendezVous_InvalidPassword = 0x0003_0065,
    RendezVous_UsernameAlreadyExists = 0x0003_0066,
    RendezVous_AccountDisabled = 0x0003_0067,
    RendezVous_AccountExpired = 0x0003_0068,
    RendezVous_ConcurrentLoginDenied = 0x0003_0069,
    RendezVous_EncryptionFailure = 0x0003_006A,
    RendezVous_InvalidPID = 0x0003_006B,
    RendezVous_MaxConnectionsReached = 0x0003_006C,
    RendezVous_InvalidGID = 0x0003_006D,
    RendezVous_InvalidControlScriptID = 0x0003_006E,
    RendezVous_InvalidOperationInLiveEnvironment = 0x0003_006F,
    RendezVous_DuplicateEntry = 0x0003_0070,
    RendezVous_ControlScriptFailure = 0x0003_0071,
    RendezVous_ClassNotFound = 0x0003_0072,
    RendezVous_SessionVoid = 0x0003_0073,
    RendezVous_DDLMismatch = 0x0003_0075,
    RendezVous_InvalidConfiguration = 0x0003_0076,
    RendezVous_SessionFull = 0x0003_00C8,
    RendezVous_InvalidGatheringPassword = 0x0003_00C9,
    RendezVous_WithoutParticipationPeriod = 0x0003_00CA,
    RendezVous_PersistentGatheringCreationMax = 0x0003_00CB,
    RendezVous_PersistentGatheringParticipationMax = 0x0003_00CC,
    RendezVous_DeniedByParticipants = 0x0003_00CD,
    RendezVous_ParticipantInBlackList = 0x0003_00CE,
    RendezVous_GameServerMaintenance = 0x0003_00CF,
    RendezVous_OperationPostpone = 0x0003_00D0,
    RendezVous_OutOfRatingRange = 0x0003_00D1,
    RendezVous_ConnectionDisconnected = 0x0003_00D2,
    RendezVous_InvalidOperation = 0x0003_00D3,
    RendezVous_NotParticipatedGathering = 0x0003_00D4,
    RendezVous_MatchmakeSessionUserPasswordUnmatch = 0x0003_00D5,
    RendezVous_MatchmakeSessionSystemPasswordUnmatch = 0x0003_00D6,
    RendezVous_UserIsOffline = 0x0003_00D7,
    RendezVous_AlreadyParticipatedGathering = 0x0003_00D8,
    RendezVous_PermissionDenied = 0x0003_00D9,
    RendezVous_NotFriend = 0x0003_00DA,
    RendezVous_SessionClosed = 0x0003_00DB,
    RendezVous_DatabaseTemporarilyUnavailable = 0x0003_00DC,
    RendezVous_InvalidUniqueId = 0x0003_00DD,
    RendezVous_MatchmakingWithdrawn = 0x0003_00DE,
    RendezVous_LimitExceeded = 0x0003_00DF,
    RendezVous_AccountTemporarilyDisabled = 0x0003_00E0,
    RendezVous_PartiallyServiceClosed = 0x0003_00E1,
    RendezVous_ConnectionDisconnectedForConcurrentLogin = 0x0003_00E2,
    PythonCore_Exception = 0x0004_0001,
    PythonCore_TypeError = 0x0004_0002,
    PythonCore_IndexError = 0x0004_0003,
    PythonCore_InvalidReference = 0x0004_0004,
    PythonCore_CallFailure = 0x0004_0005,
    PythonCore_MemoryError = 0x0004_0006,
    PythonCore_KeyError = 0x0004_0007,
    PythonCore_OperationError = 0x0004_0008,
    PythonCore_ConversionError = 0x0004_0009,
    PythonCore_ValidationError = 0x0004_000A,
    Transport_Unknown = 0x0005_0001,
    Transport_ConnectionFailure = 0x0005_0002,
    Transport_InvalidUrl = 0x0005_0003,
    Transport_InvalidKey = 0x0005_0004,
    Transport_InvalidURLType = 0x0005_0005,
    Transport_DuplicateEndpoint = 0x0005_0006,
    Transport_IOError = 0x0005_0007,
    Transport_Timeout = 0x0005_0008,
    Transport_ConnectionReset = 0x0005_0009,
    Transport_IncorrectRemoteAuthentication = 0x0005_000A,
    Transport_ServerRequestError = 0x0005_000B,
    Transport_DecompressionFailure = 0x0005_000C,
    Transport_ReliableSendBufferFullFatal = 0x0005_000D,
    Transport_UPnPCannotInit = 0x0005_000E,
    Transport_UPnPCannotAddMapping = 0x0005_000F,
    Transport_NatPMPCannotInit = 0x0005_0010,
    Transport_NatPMPCannotAddMapping = 0x0005_0011,
    Transport_UnsupportedNAT = 0x0005_0013,
    Transport_DnsError = 0x0005_0014,
    Transport_ProxyError = 0x0005_0015,
    Transport_DataRemaining = 0x0005_0016,
    Transport_NoBuffer = 0x0005_0017,
    Transport_NotFound = 0x0005_0018,
    Transport_TemporaryServerError = 0x0005_0019,
    Transport_PermanentServerError = 0x0005_001A,
    Transport_ServiceUnavailable = 0x0005_001B,
    Transport_ReliableSendBufferFull = 0x0005_001C,
    Transport_InvalidStation = 0x0005_001D,
    Transport_InvalidSubStreamID = 0x0005_001E,
    Transport_PacketBufferFull = 0x0005_001F,
    Transport_NatTraversalError = 0x0005_0020,
    Transport_NatCheckError = 0x0005_0021,
    DOCore_StationNotReached = 0x0006_0001,
    DOCore_TargetStationDisconnect = 0x0006_0002,
    DOCore_LocalStationLeaving = 0x0006_0003,
    DOCore_ObjectNotFound = 0x0006_0004,
    DOCore_InvalidRole = 0x0006_0005,
    DOCore_CallTimeout = 0x0006_0006,
    DOCore_RMCDispatchFailed = 0x0006_0007,
    DOCore_MigrationInProgress = 0x0006_0008,
    DOCore_NoAuthority = 0x0006_0009,
    DOCore_NoTargetStationSpecified = 0x0006_000A,
    DOCore_JoinFailed = 0x0006_000B,
    DOCore_JoinDenied = 0x0006_000C,
    DOCore_ConnectivityTestFailed = 0x0006_000D,
    DOCore_Unknown = 0x0006_000E,
    DOCore_UnfreedReferences = 0x0006_000F,
    DOCore_JobTerminationFailed = 0x0006_0010,
    DOCore_InvalidState = 0x0006_0011,
    DOCore_FaultRecoveryFatal = 0x0006_0012,
    DOCore_FaultRecoveryJobProcessFailed = 0x0006_0013,
    DOCore_StationInconsitency = 0x0006_0014,
    DOCore_AbnormalMasterState = 0x0006_0015,
    DOCore_VersionMismatch = 0x0006_0016,
    FPD_NotInitialized = 0x0065_0000,
    FPD_AlreadyInitialized = 0x0065_0001,
    FPD_NotConnected = 0x0065_0002,
    FPD_Connected = 0x0065_0003,
    FPD_InitializationFailure = 0x0065_0004,
    FPD_OutOfMemory = 0x0065_0005,
    FPD_RmcFailed = 0x0065_0006,
    FPD_InvalidArgument = 0x0065_0007,
    FPD_InvalidLocalAccountID = 0x0065_0008,
    FPD_InvalidPrincipalID = 0x0065_0009,
    FPD_InvalidLocalFriendCode = 0x0065_000A,
    FPD_LocalAccountNotExists = 0x0065_000B,
    FPD_LocalAccountNotLoaded = 0x0065_000C,
    FPD_LocalAccountAlreadyLoaded = 0x0065_000D,
    FPD_FriendAlreadyExists = 0x0065_000E,
    FPD_FriendNotExists = 0x0065_000F,
    FPD_FriendNumMax = 0x0065_0010,
    FPD_NotFriend = 0x0065_0011,
    FPD_FileIO = 0x0065_0012,
    FPD_P2PInternetProhibited = 0x0065_0013,
    FPD_Unknown = 0x0065_0014,
    FPD_InvalidState = 0x0065_0015,
    FPD_AddFriendProhibited = 0x0065_0017,
    FPD_InvalidAccount = 0x0065_0019,
    FPD_BlacklistedByMe = 0x0065_001A,
    FPD_FriendAlreadyAdded = 0x0065_001C,
    FPD_MyFriendListLimitExceed = 0x0065_001D,
    FPD_RequestLimitExceed = 0x0065_001E,
    FPD_InvalidMessageID = 0x0065_001F,
    FPD_MessageIsNotMine = 0x0065_0020,
    FPD_MessageIsNotForMe = 0x0065_0021,
    FPD_FriendRequestBlocked = 0x0065_0022,
    FPD_NotInMyFriendList = 0x0065_0023,
    FPD_FriendListedByMe = 0x0065_0024,
    FPD_NotInMyBlacklist = 0x0065_0025,
    FPD_IncompatibleAccount = 0x0065_0026,
    FPD_BlockSettingChangeNotAllowed = 0x0065_0027,
    FPD_SizeLimitExceeded = 0x0065_0028,
    FPD_OperationNotAllowed = 0x0065_0029,
    FPD_NotNetworkAccount = 0x0065_002A,
    FPD_NotificationNotFound = 0x0065_002B,
    FPD_PreferenceNotInitialized = 0x0065_002C,
    FPD_FriendRequestNotAllowed = 0x0065_002D,
    Ranking_NotInitialized = 0x0067_0001,
    Ranking_InvalidArgument = 0x0067_0002,
    Ranking_RegistrationError = 0x0067_0003,
    Ranking_NotFound = 0x0067_0005,
    Ranking_InvalidScore = 0x0067_0006,
    Ranking_InvalidDataSize = 0x0067_0007,
    Ranking_PermissionDenied = 0x0067_0009,
    Ranking_Unknown = 0x0067_000A,
    Ranking_NotImplemented = 0x0067_000B,
    Authentication_NASAuthenticateError = 0x0068_0001,
    Authentication_TokenParseError = 0x0068_0002,
    Authentication_HttpConnectionError = 0x0068_0003,
    Authentication_HttpDNSError = 0x0068_0004,
    Authentication_HttpGetProxySetting = 0x0068_0005,
    Authentication_TokenExpired = 0x0068_0006,
    Authentication_ValidationFailed = 0x0068_0007,
    Authentication_InvalidParam = 0x0068_0008,
    Authentication_PrincipalIdUnmatched = 0x0068_0009,
    Authentication_MoveCountUnmatch = 0x0068_000A,
    Authentication_UnderMaintenance = 0x0068_000B,
    Authentication_UnsupportedVersion = 0x0068_000C,
    Authentication_ServerVersionIsOld = 0x0068_000D,
    Authentication_Unknown = 0x0068_000E,
    Authentication_ClientVersionIsOld = 0x0068_000F,
    Authentication_AccountLibraryError = 0x0068_0010,
    Authentication_ServiceNoLongerAvailable = 0x0068_0011,
    Authentication_UnknownApplication = 0x0068_0012,
    Authentication_ApplicationVersionIsOld = 0x0068_0013,
    Authentication_OutOfService = 0x0068_0014,
    Authentication_NetworkServiceLicenseRequired = 0x0068_0015,
    Authentication_NetworkServiceLicenseSystemError = 0x0068_0016,
    Authentication_NetworkServiceLicenseError3 = 0x0068_0017,
    Authentication_NetworkServiceLicenseError4 = 0x0068_0018,
    DataStore_Unknown = 0x0069_0001,
    DataStore_InvalidArgument = 0x0069_0002,
    DataStore_PermissionDenied = 0x0069_0003,
    DataStore_NotFound = 0x0069_0004,
    DataStore_AlreadyLocked = 0x0069_0005,
    DataStore_UnderReviewing = 0x0069_0006,
    DataStore_Expired = 0x0069_0007,
    DataStore_InvalidCheckToken = 0x0069_0008,
    DataStore_SystemFileError = 0x0069_0009,
    DataStore_OverCapacity = 0x0069_000A,
    DataStore_OperationNotAllowed = 0x0069_000B,
    DataStore_InvalidPassword = 0x0069_000C,
    DataStore_ValueNotEqual = 0x0069_000D,
    ServiceItem_Unknown = 0x006C_0001,
    ServiceItem_InvalidArgument = 0x006C_0002,
    ServiceItem_EShopUnknownHttpError = 0x006C_0003,
    ServiceItem_EShopResponseParseError = 0x006C_0004,
    ServiceItem_NotOwned = 0x006C_0005,
    ServiceItem_InvalidLimitationType = 0x006C_0006,
    ServiceItem_ConsumptionRightShortage = 0x006C_0007,
    MatchmakeReferee_Unknown = 0x006F_0001,
    MatchmakeReferee_InvalidArgument = 0x006F_0002,
    MatchmakeReferee_AlreadyExists = 0x006F_0003,
    MatchmakeReferee_NotParticipatedGathering = 0x006F_0004,
    MatchmakeReferee_NotParticipatedRound = 0x006F_0005,
    MatchmakeReferee_StatsNotFound = 0x006F_0006,
    MatchmakeReferee_RoundNotFound = 0x006F_0007,
    MatchmakeReferee_RoundArbitrated = 0x006F_0008,
    MatchmakeReferee_RoundNotArbitrated = 0x006F_0009,
    Subscriber_Unknown = 0x0070_0001,
    Subscriber_InvalidArgument = 0x0070_0002,
    Subscriber_OverLimit = 0x0070_0003,
    Subscriber_PermissionDenied = 0x0070_0004,
    Ranking2_Unknown = 0x0071_0001,
    Ranking2_InvalidArgument = 0x0071_0002,
    Ranking2_InvalidScore = 0x0071_0003,
    SmartDeviceVoiceChat_Unknown = 0x0072_0001,
    SmartDeviceVoiceChat_InvalidArgument = 0x0072_0002,
    SmartDeviceVoiceChat_InvalidResponse = 0x0072_0003,
    SmartDeviceVoiceChat_InvalidAccessToken = 0x0072_0004,
    SmartDeviceVoiceChat_Unauthorized = 0x0072_0005,
    SmartDeviceVoiceChat_AccessError = 0x0072_0006,
    SmartDeviceVoiceChat_UserNotFound = 0x0072_0007,
    SmartDeviceVoiceChat_RoomNotFound = 0x0072_0008,
    SmartDeviceVoiceChat_RoomNotActivated = 0x0072_0009,
    SmartDeviceVoiceChat_ApplicationNotSupported = 0x0072_000A,
    SmartDeviceVoiceChat_InternalServerError = 0x0072_000B,
    SmartDeviceVoiceChat_ServiceUnavailable = 0x0072_000C,
    SmartDeviceVoiceChat_UnexpectedError = 0x0072_000D,
    SmartDeviceVoiceChat_UnderMaintenance = 0x0072_000E,
    SmartDeviceVoiceChat_ServiceNoLongerAvailable = 0x0072_000F,
    SmartDeviceVoiceChat_AccountTemporarilyDisabled = 0x0072_0010,
    SmartDeviceVoiceChat_PermissionDenied = 0x0072_0011,
    SmartDeviceVoiceChat_NetworkServiceLicenseRequired = 0x0072_0012,
    SmartDeviceVoiceChat_AccountLibraryError = 0x0072_0013,
    SmartDeviceVoiceChat_GameModeNotFound = 0x0072_0014,
    Screening_Unknown = 0x0073_0001,
    Screening_InvalidArgument = 0x0073_0002,
    Screening_NotFound = 0x0073_0003,
    Custom_Unknown = 0x0074_0001,
    Ess_Unknown = 0x0075_0001,
    Ess_GameSessionError = 0x0075_0002,
    Ess_GameSessionMaintenance = 0x0075_0003,
}

impl TryFrom<u32> for ErrorCode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            value if value == Self::Core_Unknown as u32 => Ok(Self::Core_Unknown),
            value if value == Self::Core_NotImplemented as u32 => Ok(Self::Core_NotImplemented),
            value if value == Self::Core_InvalidPointer as u32 => Ok(Self::Core_InvalidPointer),
            value if value == Self::Core_OperationAborted as u32 => Ok(Self::Core_OperationAborted),
            value if value == Self::Core_Exception as u32 => Ok(Self::Core_Exception),
            value if value == Self::Core_AccessDenied as u32 => Ok(Self::Core_AccessDenied),
            value if value == Self::Core_InvalidHandle as u32 => Ok(Self::Core_InvalidHandle),
            value if value == Self::Core_InvalidIndex as u32 => Ok(Self::Core_InvalidIndex),
            value if value == Self::Core_OutOfMemory as u32 => Ok(Self::Core_OutOfMemory),
            value if value == Self::Core_InvalidArgument as u32 => Ok(Self::Core_InvalidArgument),
            value if value == Self::Core_Timeout as u32 => Ok(Self::Core_Timeout),
            value if value == Self::Core_InitializationFailure as u32 => Ok(Self::Core_InitializationFailure),
            value if value == Self::Core_CallInitiationFailure as u32 => Ok(Self::Core_CallInitiationFailure),
            value if value == Self::Core_RegistrationError as u32 => Ok(Self::Core_RegistrationError),
            value if value == Self::Core_BufferOverflow as u32 => Ok(Self::Core_BufferOverflow),
            value if value == Self::Core_InvalidLockState as u32 => Ok(Self::Core_InvalidLockState),
            value if value == Self::Core_InvalidSequence as u32 => Ok(Self::Core_InvalidSequence),
            value if value == Self::Core_SystemError as u32 => Ok(Self::Core_SystemError),
            value if value == Self::Core_Cancelled as u32 => Ok(Self::Core_Cancelled),
            value if value == Self::DDL_InvalidSignature as u32 => Ok(Self::DDL_InvalidSignature),
            value if value == Self::DDL_IncorrectVersion as u32 => Ok(Self::DDL_IncorrectVersion),
            value if value == Self::RendezVous_ConnectionFailure as u32 => Ok(Self::RendezVous_ConnectionFailure),
            value if value == Self::RendezVous_NotAuthenticated as u32 => Ok(Self::RendezVous_NotAuthenticated),
            value if value == Self::RendezVous_InvalidUsername as u32 => Ok(Self::RendezVous_InvalidUsername),
            value if value == Self::RendezVous_InvalidPassword as u32 => Ok(Self::RendezVous_InvalidPassword),
            value if value == Self::RendezVous_UsernameAlreadyExists as u32 => Ok(Self::RendezVous_UsernameAlreadyExists),
            value if value == Self::RendezVous_AccountDisabled as u32 => Ok(Self::RendezVous_AccountDisabled),
            value if value == Self::RendezVous_AccountExpired as u32 => Ok(Self::RendezVous_AccountExpired),
            value if value == Self::RendezVous_ConcurrentLoginDenied as u32 => Ok(Self::RendezVous_ConcurrentLoginDenied),
            value if value == Self::RendezVous_EncryptionFailure as u32 => Ok(Self::RendezVous_EncryptionFailure),
            value if value == Self::RendezVous_InvalidPID as u32 => Ok(Self::RendezVous_InvalidPID),
            value if value == Self::RendezVous_MaxConnectionsReached as u32 => Ok(Self::RendezVous_MaxConnectionsReached),
            value if value == Self::RendezVous_InvalidGID as u32 => Ok(Self::RendezVous_InvalidGID),
            value if value == Self::RendezVous_InvalidControlScriptID as u32 => Ok(Self::RendezVous_InvalidControlScriptID),
            value if value == Self::RendezVous_InvalidOperationInLiveEnvironment as u32 => Ok(Self::RendezVous_InvalidOperationInLiveEnvironment),
            value if value == Self::RendezVous_DuplicateEntry as u32 => Ok(Self::RendezVous_DuplicateEntry),
            value if value == Self::RendezVous_ControlScriptFailure as u32 => Ok(Self::RendezVous_ControlScriptFailure),
            value if value == Self::RendezVous_ClassNotFound as u32 => Ok(Self::RendezVous_ClassNotFound),
            value if value == Self::RendezVous_SessionVoid as u32 => Ok(Self::RendezVous_SessionVoid),
            value if value == Self::RendezVous_DDLMismatch as u32 => Ok(Self::RendezVous_DDLMismatch),
            value if value == Self::RendezVous_InvalidConfiguration as u32 => Ok(Self::RendezVous_InvalidConfiguration),
            value if value == Self::RendezVous_SessionFull as u32 => Ok(Self::RendezVous_SessionFull),
            value if value == Self::RendezVous_InvalidGatheringPassword as u32 => Ok(Self::RendezVous_InvalidGatheringPassword),
            value if value == Self::RendezVous_WithoutParticipationPeriod as u32 => Ok(Self::RendezVous_WithoutParticipationPeriod),
            value if value == Self::RendezVous_PersistentGatheringCreationMax as u32 => Ok(Self::RendezVous_PersistentGatheringCreationMax),
            value if value == Self::RendezVous_PersistentGatheringParticipationMax as u32 => Ok(Self::RendezVous_PersistentGatheringParticipationMax),
            value if value == Self::RendezVous_DeniedByParticipants as u32 => Ok(Self::RendezVous_DeniedByParticipants),
            value if value == Self::RendezVous_ParticipantInBlackList as u32 => Ok(Self::RendezVous_ParticipantInBlackList),
            value if value == Self::RendezVous_GameServerMaintenance as u32 => Ok(Self::RendezVous_GameServerMaintenance),
            value if value == Self::RendezVous_OperationPostpone as u32 => Ok(Self::RendezVous_OperationPostpone),
            value if value == Self::RendezVous_OutOfRatingRange as u32 => Ok(Self::RendezVous_OutOfRatingRange),
            value if value == Self::RendezVous_ConnectionDisconnected as u32 => Ok(Self::RendezVous_ConnectionDisconnected),
            value if value == Self::RendezVous_InvalidOperation as u32 => Ok(Self::RendezVous_InvalidOperation),
            value if value == Self::RendezVous_NotParticipatedGathering as u32 => Ok(Self::RendezVous_NotParticipatedGathering),
            value if value == Self::RendezVous_MatchmakeSessionUserPasswordUnmatch as u32 => Ok(Self::RendezVous_MatchmakeSessionUserPasswordUnmatch),
            value if value == Self::RendezVous_MatchmakeSessionSystemPasswordUnmatch as u32 => Ok(Self::RendezVous_MatchmakeSessionSystemPasswordUnmatch),
            value if value == Self::RendezVous_UserIsOffline as u32 => Ok(Self::RendezVous_UserIsOffline),
            value if value == Self::RendezVous_AlreadyParticipatedGathering as u32 => Ok(Self::RendezVous_AlreadyParticipatedGathering),
            value if value == Self::RendezVous_PermissionDenied as u32 => Ok(Self::RendezVous_PermissionDenied),
            value if value == Self::RendezVous_NotFriend as u32 => Ok(Self::RendezVous_NotFriend),
            value if value == Self::RendezVous_SessionClosed as u32 => Ok(Self::RendezVous_SessionClosed),
            value if value == Self::RendezVous_DatabaseTemporarilyUnavailable as u32 => Ok(Self::RendezVous_DatabaseTemporarilyUnavailable),
            value if value == Self::RendezVous_InvalidUniqueId as u32 => Ok(Self::RendezVous_InvalidUniqueId),
            value if value == Self::RendezVous_MatchmakingWithdrawn as u32 => Ok(Self::RendezVous_MatchmakingWithdrawn),
            value if value == Self::RendezVous_LimitExceeded as u32 => Ok(Self::RendezVous_LimitExceeded),
            value if value == Self::RendezVous_AccountTemporarilyDisabled as u32 => Ok(Self::RendezVous_AccountTemporarilyDisabled),
            value if value == Self::RendezVous_PartiallyServiceClosed as u32 => Ok(Self::RendezVous_PartiallyServiceClosed),
            value if value == Self::RendezVous_ConnectionDisconnectedForConcurrentLogin as u32 => Ok(Self::RendezVous_ConnectionDisconnectedForConcurrentLogin),
            value if value == Self::PythonCore_Exception as u32 => Ok(Self::PythonCore_Exception),
            value if value == Self::PythonCore_TypeError as u32 => Ok(Self::PythonCore_TypeError),
            value if value == Self::PythonCore_IndexError as u32 => Ok(Self::PythonCore_IndexError),
            value if value == Self::PythonCore_InvalidReference as u32 => Ok(Self::PythonCore_InvalidReference),
            value if value == Self::PythonCore_CallFailure as u32 => Ok(Self::PythonCore_CallFailure),
            value if value == Self::PythonCore_MemoryError as u32 => Ok(Self::PythonCore_MemoryError),
            value if value == Self::PythonCore_KeyError as u32 => Ok(Self::PythonCore_KeyError),
            value if value == Self::PythonCore_OperationError as u32 => Ok(Self::PythonCore_OperationError),
            value if value == Self::PythonCore_ConversionError as u32 => Ok(Self::PythonCore_ConversionError),
            value if value == Self::PythonCore_ValidationError as u32 => Ok(Self::PythonCore_ValidationError),
            value if value == Self::Transport_Unknown as u32 => Ok(Self::Transport_Unknown),
            value if value == Self::Transport_ConnectionFailure as u32 => Ok(Self::Transport_ConnectionFailure),
            value if value == Self::Transport_InvalidUrl as u32 => Ok(Self::Transport_InvalidUrl),
            value if value == Self::Transport_InvalidKey as u32 => Ok(Self::Transport_InvalidKey),
            value if value == Self::Transport_InvalidURLType as u32 => Ok(Self::Transport_InvalidURLType),
            value if value == Self::Transport_DuplicateEndpoint as u32 => Ok(Self::Transport_DuplicateEndpoint),
            value if value == Self::Transport_IOError as u32 => Ok(Self::Transport_IOError),
            value if value == Self::Transport_Timeout as u32 => Ok(Self::Transport_Timeout),
            value if value == Self::Transport_ConnectionReset as u32 => Ok(Self::Transport_ConnectionReset),
            value if value == Self::Transport_IncorrectRemoteAuthentication as u32 => Ok(Self::Transport_IncorrectRemoteAuthentication),
            value if value == Self::Transport_ServerRequestError as u32 => Ok(Self::Transport_ServerRequestError),
            value if value == Self::Transport_DecompressionFailure as u32 => Ok(Self::Transport_DecompressionFailure),
            value if value == Self::Transport_ReliableSendBufferFullFatal as u32 => Ok(Self::Transport_ReliableSendBufferFullFatal),
            value if value == Self::Transport_UPnPCannotInit as u32 => Ok(Self::Transport_UPnPCannotInit),
            value if value == Self::Transport_UPnPCannotAddMapping as u32 => Ok(Self::Transport_UPnPCannotAddMapping),
            value if value == Self::Transport_NatPMPCannotInit as u32 => Ok(Self::Transport_NatPMPCannotInit),
            value if value == Self::Transport_NatPMPCannotAddMapping as u32 => Ok(Self::Transport_NatPMPCannotAddMapping),
            value if value == Self::Transport_UnsupportedNAT as u32 => Ok(Self::Transport_UnsupportedNAT),
            value if value == Self::Transport_DnsError as u32 => Ok(Self::Transport_DnsError),
            value if value == Self::Transport_ProxyError as u32 => Ok(Self::Transport_ProxyError),
            value if value == Self::Transport_DataRemaining as u32 => Ok(Self::Transport_DataRemaining),
            value if value == Self::Transport_NoBuffer as u32 => Ok(Self::Transport_NoBuffer),
            value if value == Self::Transport_NotFound as u32 => Ok(Self::Transport_NotFound),
            value if value == Self::Transport_TemporaryServerError as u32 => Ok(Self::Transport_TemporaryServerError),
            value if value == Self::Transport_PermanentServerError as u32 => Ok(Self::Transport_PermanentServerError),
            value if value == Self::Transport_ServiceUnavailable as u32 => Ok(Self::Transport_ServiceUnavailable),
            value if value == Self::Transport_ReliableSendBufferFull as u32 => Ok(Self::Transport_ReliableSendBufferFull),
            value if value == Self::Transport_InvalidStation as u32 => Ok(Self::Transport_InvalidStation),
            value if value == Self::Transport_InvalidSubStreamID as u32 => Ok(Self::Transport_InvalidSubStreamID),
            value if value == Self::Transport_PacketBufferFull as u32 => Ok(Self::Transport_PacketBufferFull),
            value if value == Self::Transport_NatTraversalError as u32 => Ok(Self::Transport_NatTraversalError),
            value if value == Self::Transport_NatCheckError as u32 => Ok(Self::Transport_NatCheckError),
            value if value == Self::DOCore_StationNotReached as u32 => Ok(Self::DOCore_StationNotReached),
            value if value == Self::DOCore_TargetStationDisconnect as u32 => Ok(Self::DOCore_TargetStationDisconnect),
            value if value == Self::DOCore_LocalStationLeaving as u32 => Ok(Self::DOCore_LocalStationLeaving),
            value if value == Self::DOCore_ObjectNotFound as u32 => Ok(Self::DOCore_ObjectNotFound),
            value if value == Self::DOCore_InvalidRole as u32 => Ok(Self::DOCore_InvalidRole),
            value if value == Self::DOCore_CallTimeout as u32 => Ok(Self::DOCore_CallTimeout),
            value if value == Self::DOCore_RMCDispatchFailed as u32 => Ok(Self::DOCore_RMCDispatchFailed),
            value if value == Self::DOCore_MigrationInProgress as u32 => Ok(Self::DOCore_MigrationInProgress),
            value if value == Self::DOCore_NoAuthority as u32 => Ok(Self::DOCore_NoAuthority),
            value if value == Self::DOCore_NoTargetStationSpecified as u32 => Ok(Self::DOCore_NoTargetStationSpecified),
            value if value == Self::DOCore_JoinFailed as u32 => Ok(Self::DOCore_JoinFailed),
            value if value == Self::DOCore_JoinDenied as u32 => Ok(Self::DOCore_JoinDenied),
            value if value == Self::DOCore_ConnectivityTestFailed as u32 => Ok(Self::DOCore_ConnectivityTestFailed),
            value if value == Self::DOCore_Unknown as u32 => Ok(Self::DOCore_Unknown),
            value if value == Self::DOCore_UnfreedReferences as u32 => Ok(Self::DOCore_UnfreedReferences),
            value if value == Self::DOCore_JobTerminationFailed as u32 => Ok(Self::DOCore_JobTerminationFailed),
            value if value == Self::DOCore_InvalidState as u32 => Ok(Self::DOCore_InvalidState),
            value if value == Self::DOCore_FaultRecoveryFatal as u32 => Ok(Self::DOCore_FaultRecoveryFatal),
            value if value == Self::DOCore_FaultRecoveryJobProcessFailed as u32 => Ok(Self::DOCore_FaultRecoveryJobProcessFailed),
            value if value == Self::DOCore_StationInconsitency as u32 => Ok(Self::DOCore_StationInconsitency),
            value if value == Self::DOCore_AbnormalMasterState as u32 => Ok(Self::DOCore_AbnormalMasterState),
            value if value == Self::DOCore_VersionMismatch as u32 => Ok(Self::DOCore_VersionMismatch),
            value if value == Self::FPD_NotInitialized as u32 => Ok(Self::FPD_NotInitialized),
            value if value == Self::FPD_AlreadyInitialized as u32 => Ok(Self::FPD_AlreadyInitialized),
            value if value == Self::FPD_NotConnected as u32 => Ok(Self::FPD_NotConnected),
            value if value == Self::FPD_Connected as u32 => Ok(Self::FPD_Connected),
            value if value == Self::FPD_InitializationFailure as u32 => Ok(Self::FPD_InitializationFailure),
            value if value == Self::FPD_OutOfMemory as u32 => Ok(Self::FPD_OutOfMemory),
            value if value == Self::FPD_RmcFailed as u32 => Ok(Self::FPD_RmcFailed),
            value if value == Self::FPD_InvalidArgument as u32 => Ok(Self::FPD_InvalidArgument),
            value if value == Self::FPD_InvalidLocalAccountID as u32 => Ok(Self::FPD_InvalidLocalAccountID),
            value if value == Self::FPD_InvalidPrincipalID as u32 => Ok(Self::FPD_InvalidPrincipalID),
            value if value == Self::FPD_InvalidLocalFriendCode as u32 => Ok(Self::FPD_InvalidLocalFriendCode),
            value if value == Self::FPD_LocalAccountNotExists as u32 => Ok(Self::FPD_LocalAccountNotExists),
            value if value == Self::FPD_LocalAccountNotLoaded as u32 => Ok(Self::FPD_LocalAccountNotLoaded),
            value if value == Self::FPD_LocalAccountAlreadyLoaded as u32 => Ok(Self::FPD_LocalAccountAlreadyLoaded),
            value if value == Self::FPD_FriendAlreadyExists as u32 => Ok(Self::FPD_FriendAlreadyExists),
            value if value == Self::FPD_FriendNotExists as u32 => Ok(Self::FPD_FriendNotExists),
            value if value == Self::FPD_FriendNumMax as u32 => Ok(Self::FPD_FriendNumMax),
            value if value == Self::FPD_NotFriend as u32 => Ok(Self::FPD_NotFriend),
            value if value == Self::FPD_FileIO as u32 => Ok(Self::FPD_FileIO),
            value if value == Self::FPD_P2PInternetProhibited as u32 => Ok(Self::FPD_P2PInternetProhibited),
            value if value == Self::FPD_Unknown as u32 => Ok(Self::FPD_Unknown),
            value if value == Self::FPD_InvalidState as u32 => Ok(Self::FPD_InvalidState),
            value if value == Self::FPD_AddFriendProhibited as u32 => Ok(Self::FPD_AddFriendProhibited),
            value if value == Self::FPD_InvalidAccount as u32 => Ok(Self::FPD_InvalidAccount),
            value if value == Self::FPD_BlacklistedByMe as u32 => Ok(Self::FPD_BlacklistedByMe),
            value if value == Self::FPD_FriendAlreadyAdded as u32 => Ok(Self::FPD_FriendAlreadyAdded),
            value if value == Self::FPD_MyFriendListLimitExceed as u32 => Ok(Self::FPD_MyFriendListLimitExceed),
            value if value == Self::FPD_RequestLimitExceed as u32 => Ok(Self::FPD_RequestLimitExceed),
            value if value == Self::FPD_InvalidMessageID as u32 => Ok(Self::FPD_InvalidMessageID),
            value if value == Self::FPD_MessageIsNotMine as u32 => Ok(Self::FPD_MessageIsNotMine),
            value if value == Self::FPD_MessageIsNotForMe as u32 => Ok(Self::FPD_MessageIsNotForMe),
            value if value == Self::FPD_FriendRequestBlocked as u32 => Ok(Self::FPD_FriendRequestBlocked),
            value if value == Self::FPD_NotInMyFriendList as u32 => Ok(Self::FPD_NotInMyFriendList),
            value if value == Self::FPD_FriendListedByMe as u32 => Ok(Self::FPD_FriendListedByMe),
            value if value == Self::FPD_NotInMyBlacklist as u32 => Ok(Self::FPD_NotInMyBlacklist),
            value if value == Self::FPD_IncompatibleAccount as u32 => Ok(Self::FPD_IncompatibleAccount),
            value if value == Self::FPD_BlockSettingChangeNotAllowed as u32 => Ok(Self::FPD_BlockSettingChangeNotAllowed),
            value if value == Self::FPD_SizeLimitExceeded as u32 => Ok(Self::FPD_SizeLimitExceeded),
            value if value == Self::FPD_OperationNotAllowed as u32 => Ok(Self::FPD_OperationNotAllowed),
            value if value == Self::FPD_NotNetworkAccount as u32 => Ok(Self::FPD_NotNetworkAccount),
            value if value == Self::FPD_NotificationNotFound as u32 => Ok(Self::FPD_NotificationNotFound),
            value if value == Self::FPD_PreferenceNotInitialized as u32 => Ok(Self::FPD_PreferenceNotInitialized),
            value if value == Self::FPD_FriendRequestNotAllowed as u32 => Ok(Self::FPD_FriendRequestNotAllowed),
            value if value == Self::Ranking_NotInitialized as u32 => Ok(Self::Ranking_NotInitialized),
            value if value == Self::Ranking_InvalidArgument as u32 => Ok(Self::Ranking_InvalidArgument),
            value if value == Self::Ranking_RegistrationError as u32 => Ok(Self::Ranking_RegistrationError),
            value if value == Self::Ranking_NotFound as u32 => Ok(Self::Ranking_NotFound),
            value if value == Self::Ranking_InvalidScore as u32 => Ok(Self::Ranking_InvalidScore),
            value if value == Self::Ranking_InvalidDataSize as u32 => Ok(Self::Ranking_InvalidDataSize),
            value if value == Self::Ranking_PermissionDenied as u32 => Ok(Self::Ranking_PermissionDenied),
            value if value == Self::Ranking_Unknown as u32 => Ok(Self::Ranking_Unknown),
            value if value == Self::Ranking_NotImplemented as u32 => Ok(Self::Ranking_NotImplemented),
            value if value == Self::Authentication_NASAuthenticateError as u32 => Ok(Self::Authentication_NASAuthenticateError),
            value if value == Self::Authentication_TokenParseError as u32 => Ok(Self::Authentication_TokenParseError),
            value if value == Self::Authentication_HttpConnectionError as u32 => Ok(Self::Authentication_HttpConnectionError),
            value if value == Self::Authentication_HttpDNSError as u32 => Ok(Self::Authentication_HttpDNSError),
            value if value == Self::Authentication_HttpGetProxySetting as u32 => Ok(Self::Authentication_HttpGetProxySetting),
            value if value == Self::Authentication_TokenExpired as u32 => Ok(Self::Authentication_TokenExpired),
            value if value == Self::Authentication_ValidationFailed as u32 => Ok(Self::Authentication_ValidationFailed),
            value if value == Self::Authentication_InvalidParam as u32 => Ok(Self::Authentication_InvalidParam),
            value if value == Self::Authentication_PrincipalIdUnmatched as u32 => Ok(Self::Authentication_PrincipalIdUnmatched),
            value if value == Self::Authentication_MoveCountUnmatch as u32 => Ok(Self::Authentication_MoveCountUnmatch),
            value if value == Self::Authentication_UnderMaintenance as u32 => Ok(Self::Authentication_UnderMaintenance),
            value if value == Self::Authentication_UnsupportedVersion as u32 => Ok(Self::Authentication_UnsupportedVersion),
            value if value == Self::Authentication_ServerVersionIsOld as u32 => Ok(Self::Authentication_ServerVersionIsOld),
            value if value == Self::Authentication_Unknown as u32 => Ok(Self::Authentication_Unknown),
            value if value == Self::Authentication_ClientVersionIsOld as u32 => Ok(Self::Authentication_ClientVersionIsOld),
            value if value == Self::Authentication_AccountLibraryError as u32 => Ok(Self::Authentication_AccountLibraryError),
            value if value == Self::Authentication_ServiceNoLongerAvailable as u32 => Ok(Self::Authentication_ServiceNoLongerAvailable),
            value if value == Self::Authentication_UnknownApplication as u32 => Ok(Self::Authentication_UnknownApplication),
            value if value == Self::Authentication_ApplicationVersionIsOld as u32 => Ok(Self::Authentication_ApplicationVersionIsOld),
            value if value == Self::Authentication_OutOfService as u32 => Ok(Self::Authentication_OutOfService),
            value if value == Self::Authentication_NetworkServiceLicenseRequired as u32 => Ok(Self::Authentication_NetworkServiceLicenseRequired),
            value if value == Self::Authentication_NetworkServiceLicenseSystemError as u32 => Ok(Self::Authentication_NetworkServiceLicenseSystemError),
            value if value == Self::Authentication_NetworkServiceLicenseError3 as u32 => Ok(Self::Authentication_NetworkServiceLicenseError3),
            value if value == Self::Authentication_NetworkServiceLicenseError4 as u32 => Ok(Self::Authentication_NetworkServiceLicenseError4),
            value if value == Self::DataStore_Unknown as u32 => Ok(Self::DataStore_Unknown),
            value if value == Self::DataStore_InvalidArgument as u32 => Ok(Self::DataStore_InvalidArgument),
            value if value == Self::DataStore_PermissionDenied as u32 => Ok(Self::DataStore_PermissionDenied),
            value if value == Self::DataStore_NotFound as u32 => Ok(Self::DataStore_NotFound),
            value if value == Self::DataStore_AlreadyLocked as u32 => Ok(Self::DataStore_AlreadyLocked),
            value if value == Self::DataStore_UnderReviewing as u32 => Ok(Self::DataStore_UnderReviewing),
            value if value == Self::DataStore_Expired as u32 => Ok(Self::DataStore_Expired),
            value if value == Self::DataStore_InvalidCheckToken as u32 => Ok(Self::DataStore_InvalidCheckToken),
            value if value == Self::DataStore_SystemFileError as u32 => Ok(Self::DataStore_SystemFileError),
            value if value == Self::DataStore_OverCapacity as u32 => Ok(Self::DataStore_OverCapacity),
            value if value == Self::DataStore_OperationNotAllowed as u32 => Ok(Self::DataStore_OperationNotAllowed),
            value if value == Self::DataStore_InvalidPassword as u32 => Ok(Self::DataStore_InvalidPassword),
            value if value == Self::DataStore_ValueNotEqual as u32 => Ok(Self::DataStore_ValueNotEqual),
            value if value == Self::ServiceItem_Unknown as u32 => Ok(Self::ServiceItem_Unknown),
            value if value == Self::ServiceItem_InvalidArgument as u32 => Ok(Self::ServiceItem_InvalidArgument),
            value if value == Self::ServiceItem_EShopUnknownHttpError as u32 => Ok(Self::ServiceItem_EShopUnknownHttpError),
            value if value == Self::ServiceItem_EShopResponseParseError as u32 => Ok(Self::ServiceItem_EShopResponseParseError),
            value if value == Self::ServiceItem_NotOwned as u32 => Ok(Self::ServiceItem_NotOwned),
            value if value == Self::ServiceItem_InvalidLimitationType as u32 => Ok(Self::ServiceItem_InvalidLimitationType),
            value if value == Self::ServiceItem_ConsumptionRightShortage as u32 => Ok(Self::ServiceItem_ConsumptionRightShortage),
            value if value == Self::MatchmakeReferee_Unknown as u32 => Ok(Self::MatchmakeReferee_Unknown),
            value if value == Self::MatchmakeReferee_InvalidArgument as u32 => Ok(Self::MatchmakeReferee_InvalidArgument),
            value if value == Self::MatchmakeReferee_AlreadyExists as u32 => Ok(Self::MatchmakeReferee_AlreadyExists),
            value if value == Self::MatchmakeReferee_NotParticipatedGathering as u32 => Ok(Self::MatchmakeReferee_NotParticipatedGathering),
            value if value == Self::MatchmakeReferee_NotParticipatedRound as u32 => Ok(Self::MatchmakeReferee_NotParticipatedRound),
            value if value == Self::MatchmakeReferee_StatsNotFound as u32 => Ok(Self::MatchmakeReferee_StatsNotFound),
            value if value == Self::MatchmakeReferee_RoundNotFound as u32 => Ok(Self::MatchmakeReferee_RoundNotFound),
            value if value == Self::MatchmakeReferee_RoundArbitrated as u32 => Ok(Self::MatchmakeReferee_RoundArbitrated),
            value if value == Self::MatchmakeReferee_RoundNotArbitrated as u32 => Ok(Self::MatchmakeReferee_RoundNotArbitrated),
            value if value == Self::Subscriber_Unknown as u32 => Ok(Self::Subscriber_Unknown),
            value if value == Self::Subscriber_InvalidArgument as u32 => Ok(Self::Subscriber_InvalidArgument),
            value if value == Self::Subscriber_OverLimit as u32 => Ok(Self::Subscriber_OverLimit),
            value if value == Self::Subscriber_PermissionDenied as u32 => Ok(Self::Subscriber_PermissionDenied),
            value if value == Self::Ranking2_Unknown as u32 => Ok(Self::Ranking2_Unknown),
            value if value == Self::Ranking2_InvalidArgument as u32 => Ok(Self::Ranking2_InvalidArgument),
            value if value == Self::Ranking2_InvalidScore as u32 => Ok(Self::Ranking2_InvalidScore),
            value if value == Self::SmartDeviceVoiceChat_Unknown as u32 => Ok(Self::SmartDeviceVoiceChat_Unknown),
            value if value == Self::SmartDeviceVoiceChat_InvalidArgument as u32 => Ok(Self::SmartDeviceVoiceChat_InvalidArgument),
            value if value == Self::SmartDeviceVoiceChat_InvalidResponse as u32 => Ok(Self::SmartDeviceVoiceChat_InvalidResponse),
            value if value == Self::SmartDeviceVoiceChat_InvalidAccessToken as u32 => Ok(Self::SmartDeviceVoiceChat_InvalidAccessToken),
            value if value == Self::SmartDeviceVoiceChat_Unauthorized as u32 => Ok(Self::SmartDeviceVoiceChat_Unauthorized),
            value if value == Self::SmartDeviceVoiceChat_AccessError as u32 => Ok(Self::SmartDeviceVoiceChat_AccessError),
            value if value == Self::SmartDeviceVoiceChat_UserNotFound as u32 => Ok(Self::SmartDeviceVoiceChat_UserNotFound),
            value if value == Self::SmartDeviceVoiceChat_RoomNotFound as u32 => Ok(Self::SmartDeviceVoiceChat_RoomNotFound),
            value if value == Self::SmartDeviceVoiceChat_RoomNotActivated as u32 => Ok(Self::SmartDeviceVoiceChat_RoomNotActivated),
            value if value == Self::SmartDeviceVoiceChat_ApplicationNotSupported as u32 => Ok(Self::SmartDeviceVoiceChat_ApplicationNotSupported),
            value if value == Self::SmartDeviceVoiceChat_InternalServerError as u32 => Ok(Self::SmartDeviceVoiceChat_InternalServerError),
            value if value == Self::SmartDeviceVoiceChat_ServiceUnavailable as u32 => Ok(Self::SmartDeviceVoiceChat_ServiceUnavailable),
            value if value == Self::SmartDeviceVoiceChat_UnexpectedError as u32 => Ok(Self::SmartDeviceVoiceChat_UnexpectedError),
            value if value == Self::SmartDeviceVoiceChat_UnderMaintenance as u32 => Ok(Self::SmartDeviceVoiceChat_UnderMaintenance),
            value if value == Self::SmartDeviceVoiceChat_ServiceNoLongerAvailable as u32 => Ok(Self::SmartDeviceVoiceChat_ServiceNoLongerAvailable),
            value if value == Self::SmartDeviceVoiceChat_AccountTemporarilyDisabled as u32 => Ok(Self::SmartDeviceVoiceChat_AccountTemporarilyDisabled),
            value if value == Self::SmartDeviceVoiceChat_PermissionDenied as u32 => Ok(Self::SmartDeviceVoiceChat_PermissionDenied),
            value if value == Self::SmartDeviceVoiceChat_NetworkServiceLicenseRequired as u32 => Ok(Self::SmartDeviceVoiceChat_NetworkServiceLicenseRequired),
            value if value == Self::SmartDeviceVoiceChat_AccountLibraryError as u32 => Ok(Self::SmartDeviceVoiceChat_AccountLibraryError),
            value if value == Self::SmartDeviceVoiceChat_GameModeNotFound as u32 => Ok(Self::SmartDeviceVoiceChat_GameModeNotFound),
            value if value == Self::Screening_Unknown as u32 => Ok(Self::Screening_Unknown),
            value if value == Self::Screening_InvalidArgument as u32 => Ok(Self::Screening_InvalidArgument),
            value if value == Self::Screening_NotFound as u32 => Ok(Self::Screening_NotFound),
            value if value == Self::Custom_Unknown as u32 => Ok(Self::Custom_Unknown),
            value if value == Self::Ess_Unknown as u32 => Ok(Self::Ess_Unknown),
            value if value == Self::Ess_GameSessionError as u32 => Ok(Self::Ess_GameSessionError),
            value if value == Self::Ess_GameSessionMaintenance as u32 => Ok(Self::Ess_GameSessionMaintenance),
            _ => Err(()),
        }
    }
}

impl From<Error> for ErrorCode {
    fn from(value: Error) -> Self {
        error!("rmc error occurred during method runtime: {}", value);
        Self::Core_InvalidArgument
    }
}

impl Into<u32> for ErrorCode {
    fn into(self) -> u32 {
        self as u32
    }
}
