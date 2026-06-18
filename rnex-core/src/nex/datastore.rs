use futures::TryStreamExt;
use rnex_core::nex::user::User;
use rnex_core::PID;
use rnex_core::executables::common::{
    RNEX_DATASTORE_S3_BUCKET, RNEX_DATASTORE_S3_ENDPOINT, get_db,
};
use rnex_core::kerberos::KerberosDateTime;
use rnex_core::nex::s3presigner::S3Presigner;
use rnex_core::rmc::protocols::datastore::{BufferQueueParam, CompletePostParam, DataStoreChangeMetaParam, DataStoreCustomRankingResult, DataStoreGetCustomRankingByDataIDParam, DataStorePrepareGetParam, DataStoreReqGetInfo, DataStoreSearchParam, GetMetaInfo, GetMetaParam, KeyValue, Permission, PersistenceTarget, RateCustomRankingParam, RatingInfo, RatingInfoWithSlot, RatingInitParamWithSlot};
use rnex_core::rmc::protocols::datastore::{DataStore, PreparePostParam, ReqPostInfo, AttachFileParam, DataStoreRateObjectParam, DataStoreRatingTarget};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::qbuffer::QBuffer;
use rnex_core::rmc::structures::qresult::QResult;
use sqlx::types::time;
use crate::rmc::protocols::datastore::{DataStoreGetCourseRecordParam, DataStoreGetCourseRecordResult, DataStoreUploadCourseRecordParam};

fn map_row_to_meta_info(
    row_data_id: i64,
    row_owner: i32,
    row_size: i32,
    row_name: String,
    row_data_type: i16,
    row_meta_binary: Vec<u8>,
    row_permission: i16,
    row_permission_recipients: Vec<i64>,
    row_delete_permission: i16,
    row_delete_permission_recipients: Vec<i64>,
    row_period: i16,
    row_refer_data_id: i64,
    row_flag: i32,
    row_tags: Vec<String>,
    row_creation_date: chrono::NaiveDateTime,
    row_update_date: chrono::NaiveDateTime,
    ratings: Vec<RatingInfoWithSlot>,
) -> GetMetaInfo {
    GetMetaInfo {
        dataid: row_data_id,
        owner: row_owner as PID,
        size: row_size as u32,
        name: row_name,
        data_type: row_data_type as u16,
        meta_binary: QBuffer(row_meta_binary),
        permission: Permission {
            permission: row_permission as u8,
            recipient_ids: row_permission_recipients
                .into_iter()
                .map(|id| id as PID)
                .collect(),
        },
        del_permission: Permission {
            permission: row_delete_permission as u8,
            recipient_ids: row_delete_permission_recipients
                .into_iter()
                .map(|id| id as PID)
                .collect(),
        },
        period: row_period as u16,
        status: 0,
        referred_count: 0,
        refer_dat_id: row_refer_data_id as u32,
        flag: row_flag as u32,
        tags: row_tags,
        expire_time: KerberosDateTime::from_i64(0x9C3F3E0000),
        created_time: KerberosDateTime::from_naive(row_creation_date),
        updated_time: KerberosDateTime::from_naive(row_update_date),
        referred_time: KerberosDateTime::from_naive(row_creation_date),
        ratings,
    }
}


pub async fn check_object_availability(data_id: i64, password: i64) -> Result<(), ErrorCode> {
    let row = sqlx::query!(
        r#"
                SELECT under_review, access_password
                FROM datastore.objects
                WHERE data_id = $1 AND upload_completed = TRUE AND deleted = FALSE
                "#,
        data_id
    )
    .fetch_optional(get_db())
    .await
    .map_err(|e| {
        eprintln!("Availability check DB error: {:?}", e);
        ErrorCode::DataStore_SystemFileError
    })?
    .ok_or(ErrorCode::DataStore_NotFound)?;

    let access_password = row.access_password;
    if access_password != 0 && access_password != password {
        return Err(ErrorCode::DataStore_InvalidPassword);
    }

    if row.under_review {
        return Err(ErrorCode::DataStore_UnderReviewing);
    }

    Ok(())
}

pub async fn get_object_ratings(
    data_id: i64,
    password: i64,
) -> Result<Vec<RatingInfoWithSlot>, ErrorCode> {
    check_object_availability(data_id, password).await?;

    let rows = sqlx::query!(
        r#"
                SELECT slot, total_value, count, initial_value
                FROM datastore.object_ratings
                WHERE data_id = $1
                "#,
        data_id
    )
    .fetch_all(get_db())
    .await
    .map_err(|e| {
        eprintln!("Ratings fetch error: {:?}", e);
        ErrorCode::DataStore_SystemFileError
    })?;

    let ratings = rows
        .into_iter()
        .map(|row| RatingInfoWithSlot {
            slot: row.slot as i8,
            rating: RatingInfo {
                total_value: row.total_value.unwrap_or(0),
                count: row.count as u32,
                initial_value: row.initial_value.unwrap_or(0),
            },
        })
        .collect();

    Ok(ratings)
}

pub async fn get_object_info_by_data_id(data_id: i64, password: i64) -> Result<GetMetaInfo, ErrorCode> {
    check_object_availability(data_id, password).await?;

    let row = sqlx::query!(
                r#"SELECT data_id, owner, size, name, data_type, meta_binary,
                          permission, permission_recipients, delete_permission, delete_permission_recipients,
                          period, refer_data_id, flag, tags, creation_date, update_date
                   FROM datastore.objects WHERE data_id = $1"#,
                data_id
            )
                .fetch_optional(get_db())
                .await
                .map_err(|_| ErrorCode::DataStore_SystemFileError)?
                .ok_or(ErrorCode::DataStore_NotFound)?;

    let ratings = get_object_ratings(data_id, password).await?;

    Ok(map_row_to_meta_info(
        row.data_id,
        row.owner.unwrap_or(0),
        row.size.unwrap_or(0),
        row.name,
        row.data_type.unwrap_or(0) as i16,
        row.meta_binary.unwrap_or_default(),
        row.permission.unwrap_or(0) as i16,
        row.permission_recipients
            .unwrap_or_default()
            .into_iter()
            .map(|id| id as i64)
            .collect(),
        row.delete_permission.unwrap_or(0) as i16,
        row.delete_permission_recipients
            .unwrap_or_default()
            .into_iter()
            .map(|id| id as i64)
            .collect(),
        row.period.unwrap_or(0) as i16,
        row.refer_data_id.unwrap_or(0),
        row.flag.unwrap_or(0),
        row.tags.unwrap_or_default(),
        row.creation_date
            .map(|dt| {
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
                        .unwrap(),
                    chrono::NaiveTime::from_hms_opt(
                        dt.hour() as u32,
                        dt.minute() as u32,
                        dt.second() as u32,
                    )
                    .unwrap(),
                )
            })
            .unwrap_or_default(),
        row.update_date
            .map(|dt| {
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
                        .unwrap(),
                    chrono::NaiveTime::from_hms_opt(
                        dt.hour() as u32,
                        dt.minute() as u32,
                        dt.second() as u32,
                    )
                    .unwrap(),
                )
            })
            .unwrap_or_default(),
        ratings,
    ))
}

async fn get_object_info_by_persistence_target(
    target: PersistenceTarget,
    password: i64,
) -> Result<GetMetaInfo, ErrorCode> {
    let row = sqlx::query!(
                r#"SELECT data_id, owner, size, name, data_type, meta_binary,
                          permission, permission_recipients, delete_permission, delete_permission_recipients,
                          period, refer_data_id, flag, tags, creation_date, update_date,
                          access_password, under_review
                   FROM datastore.objects
                   WHERE owner = $1 AND persistence_slot_id = $2
                   AND upload_completed = TRUE AND deleted = FALSE"#,
                target.owner as i32,
                target.persistence_slot_id as i16
            )
                .fetch_optional(get_db())
                .await
                .map_err(|_| ErrorCode::DataStore_SystemFileError)?
                .ok_or(ErrorCode::DataStore_NotFound)?;

    let db_password = row.access_password;
    if db_password != 0 && db_password != password {
        return Err(ErrorCode::DataStore_InvalidPassword);
    }

    if row.under_review {
        return Err(ErrorCode::DataStore_UnderReviewing);
    }

    let ratings = get_object_ratings(row.data_id, password).await?;

    Ok(map_row_to_meta_info(
        row.data_id,
        row.owner.unwrap_or(0),
        row.size.unwrap_or(0),
        row.name,
        row.data_type.unwrap_or(0) as i16,
        row.meta_binary.unwrap_or_default(),
        row.permission.unwrap_or(0) as i16,
        row.permission_recipients
            .unwrap_or_default()
            .into_iter()
            .map(|id| id as i64)
            .collect(),
        row.delete_permission.unwrap_or(0) as i16,
        row.delete_permission_recipients
            .unwrap_or_default()
            .into_iter()
            .map(|id| id as i64)
            .collect(),
        row.period.unwrap_or(0) as i16,
        row.refer_data_id.unwrap_or(0),
        row.flag.unwrap_or(0),
        row.tags.unwrap_or_default(),
        row.creation_date
            .map(|dt| {
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
                        .unwrap(),
                    chrono::NaiveTime::from_hms_opt(
                        dt.hour() as u32,
                        dt.minute() as u32,
                        dt.second() as u32,
                    )
                    .unwrap(),
                )
            })
            .unwrap_or_default(),
        row.update_date
            .map(|dt| {
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
                        .unwrap(),
                    chrono::NaiveTime::from_hms_opt(
                        dt.hour() as u32,
                        dt.minute() as u32,
                        dt.second() as u32,
                    )
                    .unwrap(),
                )
            })
            .unwrap_or_default(),
        ratings,
    ))
}

async fn get_buffer_queues_by_data_id_and_slot(
    data_id: i64,
    slot: u32,
) -> Result<Vec<QBuffer>, ErrorCode> {
    check_object_availability(data_id, 0).await?;

    let rows = sqlx::query!(
        r#"
                SELECT buffer
                FROM datastore.buffer_queues
                WHERE data_id = $1 AND slot = $2
                ORDER BY creation_date ASC
                "#,
        data_id,
        slot as i32
    )
    .fetch_all(get_db())
    .await
    .map_err(|e| {
        log::error!("Buffer queue fetch error: {:?}", e);
        ErrorCode::DataStore_SystemFileError
    })?;

    let buffer_queues = rows.into_iter().map(|row| QBuffer(row.buffer)).collect();

    Ok(buffer_queues)
}

async fn verify_object_permission(
    owner_id: PID,
    viewer_id: PID,
    permission: &Permission,
) -> Result<(), ErrorCode> {
    if owner_id == viewer_id {
        return Ok(());
    }

    match permission.permission {
        0 => Ok(()),                                     // All can read
        1 => Err(ErrorCode::DataStore_PermissionDenied), // Friends only, unimplemented
        2 => {
            // Recipient IDs can read
            if permission.recipient_ids.contains(&viewer_id) {
                Ok(())
            } else {
                Err(ErrorCode::DataStore_PermissionDenied)
            }
        }
        3 => Err(ErrorCode::DataStore_PermissionDenied), // Owner only, redundant
        _ => Err(ErrorCode::DataStore_InvalidArgument),  // ??? haxx0r
    }
}

fn filter_properties_by_result_option(meta_info: &mut GetMetaInfo, result_option: u8) {
    if (result_option & 0x01) == 0 {
        meta_info.meta_binary = QBuffer(Vec::new());
    }

    if (result_option & 0x04) == 0 {
        meta_info.ratings = Vec::new();
    }

    // No idea what the other things do. :shrug:
}

async fn init_object_rating_slot(data_id: i64, rating_param: RatingInitParamWithSlot) {
    log::info!("running init object rating slot");
    let row = sqlx::query!(
            r#"
            INSERT INTO datastore.object_ratings (
                data_id,
                slot,
                flag,
                internal_flag,
                lock_type,
                initial_value,
                range_min,
                range_max,
                period_hour,
                period_duration,
                total_value
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#,
            data_id,
            rating_param.slot as i16,
            rating_param.param.flag as i16,
            rating_param.param.internal_flag as i16,
            rating_param.param.lock_type as i16,
            rating_param.param.initial_value,
            rating_param.param.range_min,
            rating_param.param.range_max,
            rating_param.param.period_hour as i16,
            rating_param.param.period_duration as i32,
            rating_param.param.initial_value,
        )
        .execute(get_db())
        .await
        .map_err(|e| {
            log::error!("DB Error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        });
    log::info!("done running");
}

// Dawg...
async fn get_custom_rankings_by_data_ids(
    application_id: u32,
    data_ids: Vec<i64>,
) -> Vec<DataStoreCustomRankingResult> {
    let mut results = Vec::with_capacity(data_ids.len());

    let rows = sqlx::query!(
        r#"
                SELECT
                    rankings.data_id,
                    rankings.value
                FROM datastore.object_custom_rankings rankings
                JOIN UNNEST($1::bigint[]) WITH ORDINALITY AS rows(data_id, ord)
                    ON rankings.data_id = rows.data_id
                    AND rankings.application_id = $2
                ORDER BY rows.ord
                "#,
        &data_ids.iter().map(|&id| id).collect::<Vec<i64>>(),
        application_id as i32
    )
    .fetch_all(get_db())
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            log::error!("Custom ranking query error: {:?}", e);
            return results;
        }
    };

    for row in rows {
        let data_id = row.data_id;
        let score = row.value.unwrap_or(0) as u32;

        if let Ok(meta) = get_object_info_by_data_id(data_id, 0).await {
            results.push(DataStoreCustomRankingResult {
                order: 0,
                score,
                meta_info: meta,
            });
        } else {
            log::warn!("Could not find metadata for ranked object {}", data_id);
        }
    }

    results
}

async fn get_user_course_object_ids(owner_pid: PID) -> Result<Vec<i64>, ErrorCode> {
    let rows = sqlx::query!(
        r#"
                SELECT data_id
                FROM datastore.objects
                WHERE owner = $1 AND data_type > 2 AND data_type < 50
                "#,
        owner_pid
    )
    .fetch_all(get_db())
    .await
    .map_err(|e| {
        log::error!("error fetching course IDs for PID {}: {:?}", owner_pid, e);
        ErrorCode::DataStore_SystemFileError
    })?;

    let mut valid_ids = Vec::new();
    for row in rows {
        let data_id = row.data_id;
        // always check avail
        if check_object_availability(data_id, 0).await.is_ok() {
            valid_ids.push(data_id);
        }
    }

    Ok(valid_ids)
}

fn get_blacklist_1() -> Vec<String> {
    vec![
        "けされ",
        "消され",
        "削除され",
        "リセットされ",
        "BANされ",
        "ＢＡＮされ",
        "キミのコース",
        "君のコース",
        "きみのコース",
        "い い ね",
        "遊びます",
        "地震",
        "震災",
        "被災",
        "津波",
        "バンされ",
        "い~ね",
        "震度",
        "じしん",
        "banされ",
        "くわしくは",
        "詳しくは",
        "ちんちん",
        "ち0こ",
        "bicth",
        "い.い．ね",
        "ナイ～ス",
        "い&い",
        "い-いね",
        "いぃね",
        "nigger",
        "ngger",
        "star if u",
        "Star if u",
        "Star if you",
        "star if you",
        "PENlS",
        "マンコ",
        "butthole",
        "LILI",
        "vagina",
        "vagyna",
        "うんち",
        "うんこ",
        "ウンコ",
        "Ｉｉｎｅ",
        "EENE",
        "まんこ",
        "ウンチ",
        "niglet",
        "nigglet",
        "please like",
        "きんたま",
        "Butthole",
        "llね",
        "iいね",
        "give a star",
        "ちんぽ",
        "亀頭",
        "penis",
        "ｳﾝｺ",
        "plz more stars",
        "star plz",
        "い()ね",
        "PLEASE star",
        "Bitte Sterne",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn get_blacklist_2() -> Vec<String> {
    vec![
        "ゼロから",
        "０から",
        "0から",
        "い　　い　　ね",
        "いい",
        "東日本",
        "大震",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn get_blacklist_3() -> Vec<String> {
    vec![
        "いいね",
        "下さい",
        "ください",
        "押して",
        "おして",
        "返す",
        "かえす",
        "星",
        "してくれ",
        "するよ",
        "☆くれたら",
        "☆あげます",
        "★くれたら",
        "★あげます",
        "しね",
        "ころす",
        "ころされた",
        "アナル",
        "ファック",
        "キンタマ",
        "○ね",
        "キチガイ",
        "うんこ",
        "KITIGAI",
        "金玉",
        "おっぱい",
        "☆おす",
        "☆押す",
        "★おす",
        "★押す",
        "いいする",
        "いいよ",
        "イイネ",
        "ケツ",
        "うんち",
        "かくせいざい",
        "覚せい剤",
        "シャブ",
        "きんたま",
        "ちんちん",
        "おしっこ",
        "ちんぽこ",
        "ころして",
        "グッド",
        "グット",
        "レ●プ",
        "バーカ",
        "きちがい",
        "ちんげ",
        "マンコ",
        "まんこ",
        "チンポ",
        "クズ",
        "ウンコ",
        "ナイスおねがいします",
        "penis",
        "イイね",
        "☆よろ",
        "ナイス!して",
        "ま/んこ",
        "まん/こ",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

// couldn't find a better way to do this im going crazyy
async fn rate_object(dataid: i64, slot: i8, rating_value: i32, access_password: i64) -> Result<RatingInfo, ErrorCode> {
    check_object_availability(dataid, access_password).await?;

    let rating = RatingInfo::default();

    let row = sqlx::query!(
        r#"
        UPDATE datastore.object_ratings
        SET total_value=total_value+$1, count=count+1
        WHERE data_id=$2 AND slot=$3
        RETURNING total_value, count, initial_value
        "#,
        rating_value as i64,
        dataid,
        slot as i16
    )
        .fetch_all(get_db())
        .await
        .map_err(|e| {
            log::error!("DB Error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        })?;

    Ok(rating)
}

async fn change_meta_object_check(param: &DataStoreChangeMetaParam) -> Result<(), ErrorCode> {
    let row = sqlx::query!(
                r#"
                SELECT update_password, under_review FROM datastore.objects WHERE data_id=$1 AND upload_completed=TRUE AND deleted=FALSE
                "#,
                param.dataid
            )
        .fetch_one(get_db())
        .await
        .map_err(|e| {
            log::error!("DB Error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        })?;

    if row.update_password != 0 && row.update_password != param.update_password {
        return Err(ErrorCode::DataStore_InvalidPassword);
    }

    if row.under_review {
        return Err(ErrorCode::DataStore_UnderReviewing);
    }

    Ok(())
}

async fn get_rating_with_slot_data_id(dataid: i64) -> Result<Vec<RatingInfoWithSlot>, ErrorCode> {
    check_object_availability(dataid, 0).await?;

    let rows = sqlx::query!(
        r#"
            SELECT slot, total_value, count, initial_value FROM datastore.object_ratings WHERE data_id=$1
        "#,
        dataid
    )
        .fetch_all(get_db())
        .await
        .map_err(|e| {
            log::error!("DB Error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        })?;

    let ratings = rows
        .into_iter()
        .map(|row| RatingInfoWithSlot {
            slot: row.slot as i8,
            rating: RatingInfo {
                total_value: row.total_value.unwrap_or(0),
                count: row.count as u32,
                initial_value: row.initial_value.unwrap_or(0),
            },
        })
        .collect::<Vec<RatingInfoWithSlot>>();

    Ok(ratings)
}

impl DataStore for User {
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode> {
        let mut meta_info = if metaparam.dataid != 0 {
            get_object_info_by_data_id(metaparam.dataid, metaparam.access_password).await?
        } else {
            get_object_info_by_persistence_target(
                metaparam.persistence_target,
                metaparam.access_password,
            )
            .await?
        };

        let current_pid = self.pid;
        verify_object_permission(meta_info.owner, current_pid, &meta_info.permission).await?;

        filter_properties_by_result_option(&mut meta_info, metaparam.result_option);

        Ok(meta_info)
    }

    async fn prepare_post_object(
        &self,
        postparam: PreparePostParam,
    ) -> Result<ReqPostInfo, ErrorCode> {
        let recipient_ids: Vec<i32> = postparam
            .permission
            .recipient_ids
            .iter()
            .map(|&id| id as i32)
            .collect();
        let del_recipient_ids: Vec<i32> = postparam
            .del_permission
            .recipient_ids
            .iter()
            .map(|&id| id as i32)
            .collect();
        let now = time::OffsetDateTime::now_utc();

        let row = sqlx::query!(
                        r#"
                        INSERT INTO datastore.objects (
                            owner, size, name, data_type, meta_binary,
                            permission, permission_recipients,
                            delete_permission, delete_permission_recipients,
                            flag, period, refer_data_id, tags,
                            persistence_slot_id, extra_data, creation_date, update_date
                        ) VALUES (
                            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
                        ) RETURNING data_id
                        "#,
                        self.pid as i32,
                        postparam.size as i32,
                        postparam.name,
                        postparam.data_type as i32,
                        &postparam.meta_binary.0,
                        postparam.permission.permission as i32,
                        &recipient_ids,
                        postparam.del_permission.permission as i32,
                        &del_recipient_ids,
                        postparam.flag as i32,
                        postparam.period as i32,
                        postparam.refer_data_id as i64,
                        &postparam.tags,
                        postparam.persistence_init_param.persistence_slot_id as i32,
                        &postparam.extra_data,
                        time::PrimitiveDateTime::new(now.date(), now.time()),
                        time::PrimitiveDateTime::new(now.date(), now.time())
                    )
                    .fetch_one(get_db())
                    .await
                    .map_err(|e| {
                        log::error!("DB Error: {:?}", e);
                        ErrorCode::DataStore_SystemFileError
                    })?;

        let data_id = row.data_id;
        let presigner = S3Presigner::new(
            &format!("https://{}", *RNEX_DATASTORE_S3_ENDPOINT),
            format!("{}", *RNEX_DATASTORE_S3_BUCKET),
        )
        .await;

        log::info!("param is: {:?}", postparam);
        log::info!("RIP len is: {}", postparam.rating_init_params.len());
        for rating_param in &postparam.rating_init_params {
            log::info!("running init params");
            init_object_rating_slot(data_id, rating_param.clone())
                .await
        }

        let key = format!("data/{}.bin", data_id);

        let (upload_url, fields) = presigner.generate_presigned_post(&key).await;

        let form_fields = fields
            .into_iter()
            .map(|(k, v)| KeyValue { key: k, value: v })
            .collect();

        Ok(ReqPostInfo {
            dataid: data_id,
            url: upload_url,
            request_headers: vec![],
            form_fields,
            root_ca_cert: vec![],
        })
    }

    async fn complete_post_object(
        &self,
        completeparam: CompletePostParam,
    ) -> Result<(), ErrorCode> {
        log::info!("Data ID: {:?}", completeparam.dataid);
        log::info!("Success: {:?}", completeparam.success);

        let record = sqlx::query!(
            r#"SELECT owner, under_review FROM datastore.objects WHERE data_id = $1"#,
            completeparam.dataid
        )
        .fetch_optional(get_db())
        .await
        .map_err(|e| {
            eprintln!("select error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        })?;

        let record = record.ok_or(ErrorCode::DataStore_NotFound)?;

        if record.under_review {
            return Err(ErrorCode::DataStore_UnderReviewing);
        }

        if record.owner.unwrap_or(0) as PID != self.pid {
            return Err(ErrorCode::DataStore_PermissionDenied);
        }

        if completeparam.success {
            sqlx::query!(
                r#"UPDATE datastore.objects SET upload_completed = true WHERE data_id = $1"#,
                completeparam.dataid
            )
            .execute(get_db())
            .await
            .map_err(|e| {
                eprintln!("update error: {:?}", e);
                ErrorCode::DataStore_SystemFileError
            })?;
        } else {
            return Err(ErrorCode::DataStore_InvalidArgument);
        }

        Ok(())
    }

    async fn rate_custom_ranking(
        &self,
        rankingparam: Vec<RateCustomRankingParam>,
    ) -> Result<(), ErrorCode> {
        for abcparam in rankingparam {
            let exists = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM datastore.objects WHERE data_id = $1)"#,
                abcparam.dataid
            )
            .fetch_one(get_db())
            .await
            .map_err(|_| ErrorCode::DataStore_SystemFileError)?;

            if !exists.unwrap_or(false) {
                return Err(ErrorCode::DataStore_NotFound);
            }

            sqlx::query!(
                            r#"
                            INSERT INTO datastore.object_custom_rankings (data_id, application_id, value)
                            VALUES ($1, $2, $3)
                            ON CONFLICT (data_id, application_id)
                            DO UPDATE SET value = datastore.object_custom_rankings.value + EXCLUDED.value
                            "#,
                            abcparam.dataid,
                            abcparam.appid as i32,
                            abcparam.score as i32
                        )
                        .execute(get_db())
                        .await
                        .map_err(|e| {
                            log::error!("update/insert error: {:?}", e);
                            ErrorCode::DataStore_SystemFileError
                        })?;
        }

        Ok(())
    }

    async fn get_application_config(&self, appid: u32) -> Result<Vec<i32>, ErrorCode> {
        const MAX_COURSE_UPLOADS: i32 = 100;

        let config = match appid {
            0 => vec![
                0x00000001,
                0x00000032,
                0x00000096,
                0x0000012c,
                0x000001f4,
                0x00000320,
                0x00000514,
                0x000007d0,
                0x00000bb8,
                0x00001388,
                MAX_COURSE_UPLOADS,
                0x00000014,
                0x0000001e,
                0x00000028,
                0x00000032,
                0x0000003c,
                0x00000046,
                0x00000050,
                0x0000005a,
                0x00000064,
                0x00000023,
                0x0000004b,
                0x00000023,
                0x0000004b,
                0x00000032,
                0x00000000,
                0x00000003,
                0x00000003,
                0x00000064,
                0x00000006,
                0x00000001,
                0x00000060,
                0x00000005,
                0x00000060,
                0x00000000,
                0x000007e4,
                0x00000001,
                0x00000001,
                0x0000000c,
                0x00000000,
            ],
            1 => vec![
                2, 1770179696, 1770179664, 1770179640, 1770180827, 1770180777, 1770180745,
                1770177625, 1770177590,
            ],
            2 => vec![0x000007df, 0x0000000c, 0x00000016, 0x00000005, 0x00000000],
            10 => vec![35, 75, 96, 40, 5, 6],
            _ => {
                log::error!("unknown SMM app id: {}", appid);
                return Err(ErrorCode::DataStore_Unknown);
            }
        };

        Ok(config)
    }

    async fn get_custom_ranking_by_data_id(
        &self,
        custom_ranking_param: DataStoreGetCustomRankingByDataIDParam,
    ) -> Result<(Vec<DataStoreCustomRankingResult>, Vec<QResult>), ErrorCode> {
        println!("appid: {:?}", custom_ranking_param.application_id);
        println!("dataid list: {:?}", custom_ranking_param.data_id_list);
        println!("result option: {:?}", custom_ranking_param.result_option);

        let mut ranking_results = get_custom_rankings_by_data_ids(
            custom_ranking_param.application_id,
            custom_ranking_param.data_id_list,
        )
        .await;

        let mut q_results = Vec::with_capacity(ranking_results.len());

        for result in &mut ranking_results {
            if (custom_ranking_param.result_option & 0x01) == 0 {
                result.meta_info.tags = Vec::new();
            }

            if (custom_ranking_param.result_option & 0x02) == 0 {
                result.meta_info.ratings = Vec::new();
            }

            if (custom_ranking_param.result_option & 0x04) == 0 {
                result.meta_info.meta_binary = QBuffer(Vec::new());
            }

            if (custom_ranking_param.result_option & 0x20) == 0 {
                result.score = 0;
            }

            q_results.push(QResult::success(ErrorCode::Core_Unknown));
        }

        Ok((ranking_results, q_results))
    }

    async fn get_buffer_queue(
        &self,
        bufferparam: BufferQueueParam,
    ) -> Result<Vec<QBuffer>, ErrorCode> {
        // log::info!("GetBufferQueue: dataid={}, slot={}", param.dataid, param.slot);

        let buffers =
            get_buffer_queues_by_data_id_and_slot(bufferparam.dataid, bufferparam.slot).await?;

        Ok(buffers)
    }

    async fn prepare_get_object(
        &self,
        prepare_get_param: DataStorePrepareGetParam,
    ) -> Result<DataStoreReqGetInfo, ErrorCode> {
        let meta_info = if prepare_get_param.dataid != 0 {
            log::info!("getting object by meta info");
            log::info!("Data ID: {:?}", prepare_get_param.dataid);
            get_object_info_by_data_id(prepare_get_param.dataid, prepare_get_param.access_password)
                .await?
        } else {
            log::info!("getting object by persistence info");
            get_object_info_by_persistence_target(
                prepare_get_param.persistence_target,
                prepare_get_param.access_password,
            )
            .await?
        };

        log::info!("verifying object permission");
        verify_object_permission(meta_info.owner, self.pid, &meta_info.permission).await?;

        let presigner = S3Presigner::new(
            &format!("https://{}", *RNEX_DATASTORE_S3_ENDPOINT),
            format!("{}", *RNEX_DATASTORE_S3_BUCKET),
        )
        .await;

        let key = format!("data/{}.bin", meta_info.dataid);
        let download_url = presigner.generate_presigned_get(&key);

        Ok(DataStoreReqGetInfo {
            url: download_url,
            request_headers: vec![],
            size: meta_info.size,
            root_ca_cert: vec![],
            dataid: meta_info.dataid,
        })
    }

    async fn followings_latest_course_search_object(
        &self,
        course_search_param: DataStoreSearchParam,
        _extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode> {
        let mut all_results = Vec::new();

        for &owner_pid in &course_search_param.owner_ids {
            let course_ids = get_user_course_object_ids(owner_pid).await?;

            if course_ids.is_empty() {
                continue;
            }

            let mut results = get_custom_rankings_by_data_ids(0, course_ids).await;

            // Flag 0x1: Return Tags
            // Flag 0x2: Return Ratings
            // Flag 0x4: Return MetaBinary
            // Flag 0x20: Return Score
            for res in &mut results {
                if course_search_param.result_option & 0x1 == 0 {
                    res.meta_info.tags = Vec::new();
                }
                if course_search_param.result_option & 0x2 == 0 {
                    res.meta_info.ratings = Vec::new();
                }
                if course_search_param.result_option & 0x4 == 0 {
                    res.meta_info.meta_binary =
                        rnex_core::rmc::structures::qbuffer::QBuffer(Vec::new());
                }
                if course_search_param.result_option & 0x20 == 0 {
                    res.score = 0;
                }
            }

            all_results.extend(results);
        }

        // note: we assume the client sorts the data lol

        Ok(all_results)
    }

    async fn get_application_config_string(
        &self,
        application_id: u32,
    ) -> Result<Vec<String>, ErrorCode> {
        let config = match application_id {
            128 => get_blacklist_1(),
            129 => get_blacklist_2(),
            130 => get_blacklist_3(),
            _ => {
                log::warn!(
                    "unsupported application_id in GetApplicationConfigString: {}",
                    application_id
                );
                Vec::new()
            }
        };

        Ok(config)
    }

    async fn get_metas_multiple_param(
        &self,
        params: Vec<GetMetaParam>,
    ) -> Result<(Vec<GetMetaInfo>, Vec<QResult>), ErrorCode> {
        let mut metas = Vec::with_capacity(params.len());
        let mut results = Vec::with_capacity(params.len());

        for param in params {
            let info_result = if param.dataid != 0 {
                get_object_info_by_data_id(param.dataid, param.access_password).await
            } else {
                get_object_info_by_persistence_target(
                    param.persistence_target,
                    param.access_password,
                )
                .await
            };

            match info_result {
                Ok(mut meta) => {
                    if let Err(e) = verify_object_permission(meta.owner, self.pid, &meta.permission).await
                    {
                        metas.push(GetMetaInfo::default());
                        results.push(QResult::error(e));
                    } else {
                        if param.result_option & 0x1 == 0 {
                            meta.tags = Vec::new();
                        }
                        if param.result_option & 0x2 == 0 {
                            meta.ratings = Vec::new();
                        }
                        if param.result_option & 0x4 == 0 {
                            meta.meta_binary =
                                rnex_core::rmc::structures::qbuffer::QBuffer(Vec::new());
                        }

                        metas.push(meta);
                        results.push(QResult::success(ErrorCode::Core_Unknown));
                    }
                }
                Err(e) => {
                    metas.push(GetMetaInfo::default());
                    results.push(QResult::error(e));
                }
            }
        }

        Ok((metas, results))
    }

    async fn prepare_attach_file(&self, param: AttachFileParam) -> Result<ReqPostInfo, ErrorCode> {
        let recipient_ids: Vec<i32> = param
            .post_param
            .permission
            .recipient_ids
            .iter()
            .map(|&id| id as i32)
            .collect();

        let del_recipient_ids: Vec<i32> = param
            .post_param
            .del_permission
            .recipient_ids
            .iter()
            .map(|&id| id as i32)
            .collect();

        let tags: Vec<String> = param
            .post_param
            .tags
            .iter()
            .map(|t| t.to_string())
            .collect();

        let extra_data: Vec<String> = param
            .post_param
            .extra_data
            .iter()
            .map(|e| e.to_string())
            .collect();

        let now = time::OffsetDateTime::now_utc();
        let db_now = time::PrimitiveDateTime::new(now.date(), now.time());

        let row = sqlx::query!(
            r#"
            INSERT INTO datastore.objects (
                owner, size, name, data_type, meta_binary,
                permission, permission_recipients,
                delete_permission, delete_permission_recipients,
                flag, period, refer_data_id, tags,
                persistence_slot_id, extra_data, creation_date, update_date
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
            ) RETURNING data_id
            "#,
            self.pid as i32,
            param.post_param.size as i32,
            param.post_param.name,
            param.post_param.data_type as i32,
            &param.post_param.meta_binary.0,
            param.post_param.permission.permission as i32,
            &recipient_ids,
            param.post_param.del_permission.permission as i32,
            &del_recipient_ids,
            param.post_param.flag as i32,
            param.post_param.period as i32,
            param.refer_data_id, // Data ID of the course this is attached to
            &tags,
            param.post_param.persistence_init_param.persistence_slot_id as i32,
            &extra_data,
            db_now,
            db_now
        )
            .fetch_one(get_db())
            .await
            .map_err(|e| {
                log::error!("DB Error: {:?}", e);
                ErrorCode::DataStore_SystemFileError
            })?;

        let data_id = row.data_id;

        for rating_param in &param.post_param.rating_init_params {
            log::info!("running init params");
            init_object_rating_slot(data_id, rating_param.clone())
                .await
        }

        let presigner = S3Presigner::new(
            &format!("https://{}", *RNEX_DATASTORE_S3_ENDPOINT),
            format!("{}", *RNEX_DATASTORE_S3_BUCKET),
        )
            .await;

        let key = format!("data/{}.jpg", data_id);

        let (upload_url, fields) = presigner.generate_presigned_post(&key).await;

        let form_fields = fields
            .into_iter()
            .map(|(k, v)| KeyValue { key: k, value: v })
            .collect();

        Ok(ReqPostInfo {
            dataid: data_id,
            url: upload_url,
            request_headers: vec![],
            form_fields,
            root_ca_cert: vec![],
        })
    }

    async fn complete_attach_file(&self, complete_attach_param: CompletePostParam) -> Result<String, ErrorCode> {
        log::info!("Data ID: {:?}", complete_attach_param.dataid);
        log::info!("Success: {:?}", complete_attach_param.success);

        let presigner = S3Presigner::new(
            &format!("https://{}", *RNEX_DATASTORE_S3_ENDPOINT),
            format!("{}", *RNEX_DATASTORE_S3_BUCKET),
        ).await;

        let key = format!("data/{}.jpg", complete_attach_param.dataid);
        let download_url = presigner.generate_presigned_get(&key);

        Ok(download_url)
    }

    async fn rate_objects(&self, targets: Vec<DataStoreRatingTarget>, params: Vec<DataStoreRateObjectParam>, _transactional: bool, fetch_ratings: bool) -> Result<(Vec<RatingInfo>, Vec<QResult>), ErrorCode> {
        let mut ratings: Vec<RatingInfo> = vec![];
        let results: Vec<QResult> = vec![];

        // SMM seems to work fine with this, no clue for other DTSR games
        if targets.len() != params.len() {
            return Err(ErrorCode::DataStore_OperationNotAllowed)
        }

        for (i, target) in targets.into_iter().enumerate() {
            let param = &params[i];

            log::info!("Data ID: {:?}", target.dataid);
            log::info!("Slot: {:?}", target.slot);
            log::info!("Access Password: {:?}", param.access_password);

            let object_info = get_object_info_by_data_id(target.dataid, param.access_password).await?;
            log::info!("object info get complete");
            verify_object_permission(object_info.owner, self.pid, &object_info.permission).await?;
            log::info!("object permission complete");
            let rating = rate_object(target.dataid, target.slot, param.rating_value, param.access_password).await?;
            log::info!("rating complete");

            if fetch_ratings {
                ratings.push(rating)
            }
        }

        Ok((ratings, results))
    }

    async fn change_meta(&self, param: DataStoreChangeMetaParam) -> Result<(), ErrorCode> {
        let object_info = get_object_info_by_data_id(param.dataid, 0).await?;
        verify_object_permission(object_info.owner, self.pid, &object_info.permission).await?;

        if param.modifies_flag & 0x08 != 0 {
            change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET period=$1 WHERE data_id=$2"#,
                param.period as i16,
                param.dataid
            )
                .execute(get_db())
                .await
                .map_err(|e| {
                    eprintln!("update error: {:?}", e);
                    ErrorCode::DataStore_SystemFileError
                })?;
        }

        if param.modifies_flag & 0x10 != 0 {
            change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET meta_binary=$1 WHERE data_id=$2"#,
                param.meta_binary.0,
                param.dataid
            )
                .execute(get_db())
                .await
                .map_err(|e| {
                    eprintln!("update error: {:?}", e);
                    ErrorCode::DataStore_SystemFileError
                })?;
        }

        if param.modifies_flag & 0x80 != 0 {
            change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET data_type=$1 WHERE data_id=$2"#,
                param.data_type as i16,
                param.dataid
            )
                .execute(get_db())
                .await
                .map_err(|e| {
                    eprintln!("update error: {:?}", e);
                    ErrorCode::DataStore_SystemFileError
                })?;
        }

        Ok(())
    }

    async fn recommended_course_search_object(&self, course_search_param: DataStoreSearchParam, extra_data: Vec<String>) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode> {
        let mut courses = Vec::new();

        let mut stream = sqlx::query!(
        r#"
            SELECT
                object.data_id,
                object.owner,
                object.size,
                object.name,
                object.data_type,
                object.meta_binary,
                object.permission,
                object.permission_recipients,
                object.delete_permission,
                object.delete_permission_recipients,
                object.period,
                object.refer_data_id,
                object.flag,
                object.tags,
                object.creation_date,
                object.update_date,
                ranking.value
            FROM datastore.objects object
            JOIN datastore.object_custom_rankings ranking
            ON
                object.data_id = ranking.data_id AND
                object.upload_completed = TRUE AND
                object.deleted = FALSE AND
                object.under_review = FALSE AND
                ranking.application_id = 0
            ORDER BY RANDOM()
            LIMIT 100
        "#
    )
            .fetch(get_db());

        while let Some(row) = stream.try_next().await.map_err(|e| {
            eprintln!("stream error: {:?}", e);
            ErrorCode::DataStore_SystemFileError
        })? {

            let permission = Permission {
                permission: row.permission.unwrap_or(0) as u8,
                recipient_ids: row.permission_recipients.unwrap_or_default(),
            };

            let del_permission = Permission {
                permission: row.delete_permission.unwrap_or(0) as u8,
                recipient_ids: row.delete_permission_recipients.unwrap_or_default(),
            };

            let meta_binary = row.meta_binary
                .map(|bytes| QBuffer(bytes))
                .unwrap_or_default();

            let created_time = row.creation_date
                .map(|t| KerberosDateTime::from_i64(t.assume_utc().unix_timestamp()))
                .unwrap_or_else(|| KerberosDateTime::from_i64(0));

            let updated_time = row.update_date
                .map(|t| KerberosDateTime::from_i64(t.assume_utc().unix_timestamp()))
                .unwrap_or_else(|| KerberosDateTime::from_i64(0));

            let referred_time = row.creation_date
                .map(|t| KerberosDateTime::from_i64(t.assume_utc().unix_timestamp()))
                .unwrap_or_else(|| KerberosDateTime::from_i64(0));

            let mut meta_info = GetMetaInfo {
                dataid: row.data_id,
                owner: row.owner.unwrap_or(0),
                size: row.size.unwrap_or(0) as u32,
                name: row.name,
                data_type: row.data_type.unwrap_or(0) as u16,
                meta_binary,
                permission,
                del_permission,
                period: row.period.unwrap_or(0) as u16,
                status: 0,
                referred_count: 0,
                refer_dat_id: row.refer_data_id.unwrap_or(0) as u32,
                flag: row.flag.unwrap_or(0) as u32,
                tags: row.tags.unwrap_or_default(),
                expire_time: KerberosDateTime::from_i64(0x9C3F3E0000),
                created_time,
                updated_time,
                referred_time,
                ratings: Vec::new(),
            };

            match get_rating_with_slot_data_id(row.data_id).await {
                Ok(ratings) => meta_info.ratings = ratings,
                Err(e) => return Err(e),
            }

            let course = DataStoreCustomRankingResult {
                order: 0,
                score: row.value.unwrap_or(0) as u32,
                meta_info,
            };

            courses.push(course);
        }

        Ok(courses)
    }

    async fn upload_course_record(&self, upload_course_record_param: DataStoreUploadCourseRecordParam) -> Result<(), ErrorCode> {
        let now = time::OffsetDateTime::now_utc();
        let db_now = time::PrimitiveDateTime::new(now.date(), now.time());

        let row = sqlx::query!(
            r#"
            INSERT INTO datastore.course_records (
                data_id,
                slot,
                first_pid,
                best_pid,
                best_score,
                creation_date,
                update_date
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            ) ON CONFLICT (data_id, slot) DO UPDATE
            SET best_score = CASE WHEN datastore.course_records.best_score > $5 THEN $5 ELSE datastore.course_records.best_score END,
                best_pid = CASE WHEN datastore.course_records.best_score > $5 THEN $4 ELSE datastore.course_records.best_pid END,
                update_date = CASE WHEN datastore.course_records.best_score > $5 THEN $7 ELSE datastore.course_records.update_date END
            "#,
            upload_course_record_param.dataid,
            upload_course_record_param.slot as i16,
            self.pid,
            self.pid,
            upload_course_record_param.score,
            db_now,
            db_now
        )
            .execute(get_db())
            .await
            .map_err(|e| {
                log::error!("DB Error: {:?}", e);
                ErrorCode::DataStore_SystemFileError
            })?;

        Ok(())
    }

    async fn get_course_record(&self, get_course_record_param: DataStoreGetCourseRecordParam) -> Result<DataStoreGetCourseRecordResult, ErrorCode> {
        let row = sqlx::query!(
            r#"
                SELECT
                    first_pid,
                    best_pid,
                    best_score,
                    creation_date,
                    update_date
                FROM datastore.course_records WHERE data_id=$1 AND slot=$2
            "#,
            get_course_record_param.dataid,
            get_course_record_param.slot as i16
        )
            .fetch_one(get_db())
            .await
            .map_err(|e| {
                log::error!("DB Error: {:?}", e);
                ErrorCode::DataStore_SystemFileError
            })?;

        Ok(
            DataStoreGetCourseRecordResult {
                dataid: get_course_record_param.dataid,
                slot: get_course_record_param.slot,
                first_pid: row.first_pid as u32,
                best_pid: row.best_pid as u32,
                best_score: row.best_score,
                created_time: KerberosDateTime::from_i64(0x9C3F3E0000),
                updated_time: KerberosDateTime::from_i64(0x9C3F3E0000),
            }
        )
    }
}
