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
use v_byte_helpers::EnumTryInto;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

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
#[derive(Debug, EnumTryInto, Clone, Copy)]
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
