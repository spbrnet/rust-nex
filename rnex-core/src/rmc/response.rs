// i seriously dont know why the compiler is complaining about unused parentheses in the repr
// attributes but this gets it to not complain anymore
#![allow(unused_parens)]

use std::io;
use std::io::{Read, Seek, Write};
use std::mem::transmute;
use bytemuck::bytes_of;
use log::error;
use v_byte_helpers::EnumTryInto;
use v_byte_helpers::{ReadExtensions, IS_BIG_ENDIAN};
use crate::rmc::response::ErrorCode::Core_Exception;
use crate::rmc::structures::qresult::ERROR_MASK;
use crate::util::SendingBufferConnection;

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

pub struct RMCResponse {
    pub protocol_id: u8,
    pub response_result: RMCResponseResult,
}

impl RMCResponse {
    pub fn new(stream: &mut (impl Seek + Read)) -> io::Result<Self>{
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

        let response_result = if is_success == 0x01{
            let call_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let method_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let method_id = method_id & (!0x8000);

            let mut data: Vec<u8> = vec![0u8; (size - 2 - 4 - 4) as _];

            stream.read(&mut data)?;


            RMCResponseResult::Success {
                call_id,
                method_id,
                data
            }
        } else {
            let error_code: u32 = stream.read_struct(IS_BIG_ENDIAN)?;
            let error_code = error_code & (!0x80000000);
            let call_id: u32 = stream.read_struct(IS_BIG_ENDIAN)?;

            RMCResponseResult::Error {
                error_code: {
                    match ErrorCode::try_from(error_code){
                        Ok(v) => v,
                        Err(()) => {
                            error!("invalid error code {:#010x}", error_code);
                            Core_Exception
                        }
                    }
                },
                call_id,

            }
        };

        Ok(Self{
            protocol_id,
            response_result
        })
    }

    pub fn get_call_id(&self) -> u32{
        match &self.response_result{
            RMCResponseResult::Success { call_id, ..} => *call_id,
            RMCResponseResult::Error { call_id, .. } => *call_id
        }
    }

    pub fn to_data(self) -> Vec<u8> {
        generate_response(self.protocol_id, self.response_result).expect("failed to generate response")
    }
}

pub fn generate_response(protocol_id: u8, response: RMCResponseResult) -> io::Result<Vec<u8>> {
    let size = 1 + 1 + match &response {
        RMCResponseResult::Success {
            data,
            ..
        } => 4 + 4 + data.len(),
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
            data
        } => {
            data_out.push(1);
            data_out.write_all(bytes_of(&call_id))?;
            let ored_method_id = method_id | 0x8000;
            data_out.write_all(bytes_of(&ored_method_id))?;
            data_out.write_all(&data)?;
        }
        RMCResponseResult::Error {
            call_id,
            error_code
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
            data: v
        },
        Err(e) =>
            RMCResponseResult::Error {
                call_id,
                error_code: e.into()
            }
    };

    let response = RMCResponse{
        response_result,
        protocol_id
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
    Core_Unknown = 0x00010001,
    Core_NotImplemented = 0x00010002,
    Core_InvalidPointer = 0x00010003,
    Core_OperationAborted = 0x00010004,
    Core_Exception = 0x00010005,
    Core_AccessDenied = 0x00010006,
    Core_InvalidHandle = 0x00010007,
    Core_InvalidIndex = 0x00010008,
    Core_OutOfMemory = 0x00010009,
    Core_InvalidArgument = 0x0001000A,
    Core_Timeout = 0x0001000B,
    Core_InitializationFailure = 0x0001000C,
    Core_CallInitiationFailure = 0x0001000D,
    Core_RegistrationError = 0x0001000E,
    Core_BufferOverflow = 0x0001000F,
    Core_InvalidLockState = 0x00010010,
    Core_InvalidSequence = 0x00010011,
    Core_SystemError = 0x00010012,
    Core_Cancelled = 0x00010013,
    DDL_InvalidSignature = 0x00020001,
    DDL_IncorrectVersion = 0x00020002,
    RendezVous_ConnectionFailure = 0x00030001,
    RendezVous_NotAuthenticated = 0x00030002,
    RendezVous_InvalidUsername = 0x00030064,
    RendezVous_InvalidPassword = 0x00030065,
    RendezVous_UsernameAlreadyExists = 0x00030066,
    RendezVous_AccountDisabled = 0x00030067,
    RendezVous_AccountExpired = 0x00030068,
    RendezVous_ConcurrentLoginDenied = 0x00030069,
    RendezVous_EncryptionFailure = 0x0003006A,
    RendezVous_InvalidPID = 0x0003006B,
    RendezVous_MaxConnectionsReached = 0x0003006C,
    RendezVous_InvalidGID = 0x0003006D,
    RendezVous_InvalidControlScriptID = 0x0003006E,
    RendezVous_InvalidOperationInLiveEnvironment = 0x0003006F,
    RendezVous_DuplicateEntry = 0x00030070,
    RendezVous_ControlScriptFailure = 0x00030071,
    RendezVous_ClassNotFound = 0x00030072,
    RendezVous_SessionVoid = 0x00030073,
    RendezVous_DDLMismatch = 0x00030075,
    RendezVous_InvalidConfiguration = 0x00030076,
    RendezVous_SessionFull = 0x000300C8,
    RendezVous_InvalidGatheringPassword = 0x000300C9,
    RendezVous_WithoutParticipationPeriod = 0x000300CA,
    RendezVous_PersistentGatheringCreationMax = 0x000300CB,
    RendezVous_PersistentGatheringParticipationMax = 0x000300CC,
    RendezVous_DeniedByParticipants = 0x000300CD,
    RendezVous_ParticipantInBlackList = 0x000300CE,
    RendezVous_GameServerMaintenance = 0x000300CF,
    RendezVous_OperationPostpone = 0x000300D0,
    RendezVous_OutOfRatingRange = 0x000300D1,
    RendezVous_ConnectionDisconnected = 0x000300D2,
    RendezVous_InvalidOperation = 0x000300D3,
    RendezVous_NotParticipatedGathering = 0x000300D4,
    RendezVous_MatchmakeSessionUserPasswordUnmatch = 0x000300D5,
    RendezVous_MatchmakeSessionSystemPasswordUnmatch = 0x000300D6,
    RendezVous_UserIsOffline = 0x000300D7,
    RendezVous_AlreadyParticipatedGathering = 0x000300D8,
    RendezVous_PermissionDenied = 0x000300D9,
    RendezVous_NotFriend = 0x000300DA,
    RendezVous_SessionClosed = 0x000300DB,
    RendezVous_DatabaseTemporarilyUnavailable = 0x000300DC,
    RendezVous_InvalidUniqueId = 0x000300DD,
    RendezVous_MatchmakingWithdrawn = 0x000300DE,
    RendezVous_LimitExceeded = 0x000300DF,
    RendezVous_AccountTemporarilyDisabled = 0x000300E0,
    RendezVous_PartiallyServiceClosed = 0x000300E1,
    RendezVous_ConnectionDisconnectedForConcurrentLogin = 0x000300E2,
    PythonCore_Exception = 0x00040001,
    PythonCore_TypeError = 0x00040002,
    PythonCore_IndexError = 0x00040003,
    PythonCore_InvalidReference = 0x00040004,
    PythonCore_CallFailure = 0x00040005,
    PythonCore_MemoryError = 0x00040006,
    PythonCore_KeyError = 0x00040007,
    PythonCore_OperationError = 0x00040008,
    PythonCore_ConversionError = 0x00040009,
    PythonCore_ValidationError = 0x0004000A,
    Transport_Unknown = 0x00050001,
    Transport_ConnectionFailure = 0x00050002,
    Transport_InvalidUrl = 0x00050003,
    Transport_InvalidKey = 0x00050004,
    Transport_InvalidURLType = 0x00050005,
    Transport_DuplicateEndpoint = 0x00050006,
    Transport_IOError = 0x00050007,
    Transport_Timeout = 0x00050008,
    Transport_ConnectionReset = 0x00050009,
    Transport_IncorrectRemoteAuthentication = 0x0005000A,
    Transport_ServerRequestError = 0x0005000B,
    Transport_DecompressionFailure = 0x0005000C,
    Transport_ReliableSendBufferFullFatal = 0x0005000D,
    Transport_UPnPCannotInit = 0x0005000E,
    Transport_UPnPCannotAddMapping = 0x0005000F,
    Transport_NatPMPCannotInit = 0x00050010,
    Transport_NatPMPCannotAddMapping = 0x00050011,
    Transport_UnsupportedNAT = 0x00050013,
    Transport_DnsError = 0x00050014,
    Transport_ProxyError = 0x00050015,
    Transport_DataRemaining = 0x00050016,
    Transport_NoBuffer = 0x00050017,
    Transport_NotFound = 0x00050018,
    Transport_TemporaryServerError = 0x00050019,
    Transport_PermanentServerError = 0x0005001A,
    Transport_ServiceUnavailable = 0x0005001B,
    Transport_ReliableSendBufferFull = 0x0005001C,
    Transport_InvalidStation = 0x0005001D,
    Transport_InvalidSubStreamID = 0x0005001E,
    Transport_PacketBufferFull = 0x0005001F,
    Transport_NatTraversalError = 0x00050020,
    Transport_NatCheckError = 0x00050021,
    DOCore_StationNotReached = 0x00060001,
    DOCore_TargetStationDisconnect = 0x00060002,
    DOCore_LocalStationLeaving = 0x00060003,
    DOCore_ObjectNotFound = 0x00060004,
    DOCore_InvalidRole = 0x00060005,
    DOCore_CallTimeout = 0x00060006,
    DOCore_RMCDispatchFailed = 0x00060007,
    DOCore_MigrationInProgress = 0x00060008,
    DOCore_NoAuthority = 0x00060009,
    DOCore_NoTargetStationSpecified = 0x0006000A,
    DOCore_JoinFailed = 0x0006000B,
    DOCore_JoinDenied = 0x0006000C,
    DOCore_ConnectivityTestFailed = 0x0006000D,
    DOCore_Unknown = 0x0006000E,
    DOCore_UnfreedReferences = 0x0006000F,
    DOCore_JobTerminationFailed = 0x00060010,
    DOCore_InvalidState = 0x00060011,
    DOCore_FaultRecoveryFatal = 0x00060012,
    DOCore_FaultRecoveryJobProcessFailed = 0x00060013,
    DOCore_StationInconsitency = 0x00060014,
    DOCore_AbnormalMasterState = 0x00060015,
    DOCore_VersionMismatch = 0x00060016,
    FPD_NotInitialized = 0x00650000,
    FPD_AlreadyInitialized = 0x00650001,
    FPD_NotConnected = 0x00650002,
    FPD_Connected = 0x00650003,
    FPD_InitializationFailure = 0x00650004,
    FPD_OutOfMemory = 0x00650005,
    FPD_RmcFailed = 0x00650006,
    FPD_InvalidArgument = 0x00650007,
    FPD_InvalidLocalAccountID = 0x00650008,
    FPD_InvalidPrincipalID = 0x00650009,
    FPD_InvalidLocalFriendCode = 0x0065000A,
    FPD_LocalAccountNotExists = 0x0065000B,
    FPD_LocalAccountNotLoaded = 0x0065000C,
    FPD_LocalAccountAlreadyLoaded = 0x0065000D,
    FPD_FriendAlreadyExists = 0x0065000E,
    FPD_FriendNotExists = 0x0065000F,
    FPD_FriendNumMax = 0x00650010,
    FPD_NotFriend = 0x00650011,
    FPD_FileIO = 0x00650012,
    FPD_P2PInternetProhibited = 0x00650013,
    FPD_Unknown = 0x00650014,
    FPD_InvalidState = 0x00650015,
    FPD_AddFriendProhibited = 0x00650017,
    FPD_InvalidAccount = 0x00650019,
    FPD_BlacklistedByMe = 0x0065001A,
    FPD_FriendAlreadyAdded = 0x0065001C,
    FPD_MyFriendListLimitExceed = 0x0065001D,
    FPD_RequestLimitExceed = 0x0065001E,
    FPD_InvalidMessageID = 0x0065001F,
    FPD_MessageIsNotMine = 0x00650020,
    FPD_MessageIsNotForMe = 0x00650021,
    FPD_FriendRequestBlocked = 0x00650022,
    FPD_NotInMyFriendList = 0x00650023,
    FPD_FriendListedByMe = 0x00650024,
    FPD_NotInMyBlacklist = 0x00650025,
    FPD_IncompatibleAccount = 0x00650026,
    FPD_BlockSettingChangeNotAllowed = 0x00650027,
    FPD_SizeLimitExceeded = 0x00650028,
    FPD_OperationNotAllowed = 0x00650029,
    FPD_NotNetworkAccount = 0x0065002A,
    FPD_NotificationNotFound = 0x0065002B,
    FPD_PreferenceNotInitialized = 0x0065002C,
    FPD_FriendRequestNotAllowed = 0x0065002D,
    Ranking_NotInitialized = 0x00670001,
    Ranking_InvalidArgument = 0x00670002,
    Ranking_RegistrationError = 0x00670003,
    Ranking_NotFound = 0x00670005,
    Ranking_InvalidScore = 0x00670006,
    Ranking_InvalidDataSize = 0x00670007,
    Ranking_PermissionDenied = 0x00670009,
    Ranking_Unknown = 0x0067000A,
    Ranking_NotImplemented = 0x0067000B,
    Authentication_NASAuthenticateError = 0x00680001,
    Authentication_TokenParseError = 0x00680002,
    Authentication_HttpConnectionError = 0x00680003,
    Authentication_HttpDNSError = 0x00680004,
    Authentication_HttpGetProxySetting = 0x00680005,
    Authentication_TokenExpired = 0x00680006,
    Authentication_ValidationFailed = 0x00680007,
    Authentication_InvalidParam = 0x00680008,
    Authentication_PrincipalIdUnmatched = 0x00680009,
    Authentication_MoveCountUnmatch = 0x0068000A,
    Authentication_UnderMaintenance = 0x0068000B,
    Authentication_UnsupportedVersion = 0x0068000C,
    Authentication_ServerVersionIsOld = 0x0068000D,
    Authentication_Unknown = 0x0068000E,
    Authentication_ClientVersionIsOld = 0x0068000F,
    Authentication_AccountLibraryError = 0x00680010,
    Authentication_ServiceNoLongerAvailable = 0x00680011,
    Authentication_UnknownApplication = 0x00680012,
    Authentication_ApplicationVersionIsOld = 0x00680013,
    Authentication_OutOfService = 0x00680014,
    Authentication_NetworkServiceLicenseRequired = 0x00680015,
    Authentication_NetworkServiceLicenseSystemError = 0x00680016,
    Authentication_NetworkServiceLicenseError3 = 0x00680017,
    Authentication_NetworkServiceLicenseError4 = 0x00680018,
    DataStore_Unknown = 0x00690001,
    DataStore_InvalidArgument = 0x00690002,
    DataStore_PermissionDenied = 0x00690003,
    DataStore_NotFound = 0x00690004,
    DataStore_AlreadyLocked = 0x00690005,
    DataStore_UnderReviewing = 0x00690006,
    DataStore_Expired = 0x00690007,
    DataStore_InvalidCheckToken = 0x00690008,
    DataStore_SystemFileError = 0x00690009,
    DataStore_OverCapacity = 0x0069000A,
    DataStore_OperationNotAllowed = 0x0069000B,
    DataStore_InvalidPassword = 0x0069000C,
    DataStore_ValueNotEqual = 0x0069000D,
    ServiceItem_Unknown = 0x006C0001,
    ServiceItem_InvalidArgument = 0x006C0002,
    ServiceItem_EShopUnknownHttpError = 0x006C0003,
    ServiceItem_EShopResponseParseError = 0x006C0004,
    ServiceItem_NotOwned = 0x006C0005,
    ServiceItem_InvalidLimitationType = 0x006C0006,
    ServiceItem_ConsumptionRightShortage = 0x006C0007,
    MatchmakeReferee_Unknown = 0x006F0001,
    MatchmakeReferee_InvalidArgument = 0x006F0002,
    MatchmakeReferee_AlreadyExists = 0x006F0003,
    MatchmakeReferee_NotParticipatedGathering = 0x006F0004,
    MatchmakeReferee_NotParticipatedRound = 0x006F0005,
    MatchmakeReferee_StatsNotFound = 0x006F0006,
    MatchmakeReferee_RoundNotFound = 0x006F0007,
    MatchmakeReferee_RoundArbitrated = 0x006F0008,
    MatchmakeReferee_RoundNotArbitrated = 0x006F0009,
    Subscriber_Unknown = 0x00700001,
    Subscriber_InvalidArgument = 0x00700002,
    Subscriber_OverLimit = 0x00700003,
    Subscriber_PermissionDenied = 0x00700004,
    Ranking2_Unknown = 0x00710001,
    Ranking2_InvalidArgument = 0x00710002,
    Ranking2_InvalidScore = 0x00710003,
    SmartDeviceVoiceChat_Unknown = 0x00720001,
    SmartDeviceVoiceChat_InvalidArgument = 0x00720002,
    SmartDeviceVoiceChat_InvalidResponse = 0x00720003,
    SmartDeviceVoiceChat_InvalidAccessToken = 0x00720004,
    SmartDeviceVoiceChat_Unauthorized = 0x00720005,
    SmartDeviceVoiceChat_AccessError = 0x00720006,
    SmartDeviceVoiceChat_UserNotFound = 0x00720007,
    SmartDeviceVoiceChat_RoomNotFound = 0x00720008,
    SmartDeviceVoiceChat_RoomNotActivated = 0x00720009,
    SmartDeviceVoiceChat_ApplicationNotSupported = 0x0072000A,
    SmartDeviceVoiceChat_InternalServerError = 0x0072000B,
    SmartDeviceVoiceChat_ServiceUnavailable = 0x0072000C,
    SmartDeviceVoiceChat_UnexpectedError = 0x0072000D,
    SmartDeviceVoiceChat_UnderMaintenance = 0x0072000E,
    SmartDeviceVoiceChat_ServiceNoLongerAvailable = 0x0072000F,
    SmartDeviceVoiceChat_AccountTemporarilyDisabled = 0x00720010,
    SmartDeviceVoiceChat_PermissionDenied = 0x00720011,
    SmartDeviceVoiceChat_NetworkServiceLicenseRequired = 0x00720012,
    SmartDeviceVoiceChat_AccountLibraryError = 0x00720013,
    SmartDeviceVoiceChat_GameModeNotFound = 0x00720014,
    Screening_Unknown = 0x00730001,
    Screening_InvalidArgument = 0x00730002,
    Screening_NotFound = 0x00730003,
    Custom_Unknown = 0x00740001,
    Ess_Unknown = 0x00750001,
    Ess_GameSessionError = 0x00750002,
    Ess_GameSessionMaintenance = 0x00750003,
}

impl Into<u32> for ErrorCode {
    fn into(self) -> u32 {
        unsafe { transmute(self) }
    }
}

#[cfg(test)]
mod test {
    use hmac::digest::consts::U5;
    use hmac::digest::KeyInit;
    use rc4::{Rc4, StreamCipher};
    use crate::rmc::response::ErrorCode;

    #[test]
    fn test() {
        let data_orig = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 69, 4, 20];
        let mut data = data_orig;

        let mut rc4: Rc4<U5> =
            Rc4::new_from_slice("FUCKE".as_bytes().into()).expect("invalid key");

        rc4.apply_keystream(&mut data);

        assert_ne!(data_orig, data);

        let mut rc4: Rc4<U5> =
            Rc4::new_from_slice("FUCKE".as_bytes().into()).expect("invalid key");

        rc4.apply_keystream(&mut data);

        assert_eq!(data_orig, data);
    }

    #[test]
    fn test_enum_equivilance() {
        let val: u32 = ErrorCode::Core_Unknown.into();
        assert_eq!(val, 0x00010001)
    }
}