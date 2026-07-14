use chrono::Utc;
use futures::{TryStreamExt, future::join_all};
use rnex_base::user::BaseUser;
use rnex_ds_protos::{
    LocalDatastoreProtocol,
    datastore::{
        AttachFileParam, BufferQueueParam, CompletePostParam, DataStore, DataStoreChangeMetaParam,
        DataStoreCustomRankingResult, DataStoreDeleteParam, DataStoreFileServerObjectInfo,
        DataStoreGetCourseRecordParam, DataStoreGetCourseRecordResult,
        DataStoreGetCustomRankingByDataIDParam, DataStoreGetCustomRankingParam,
        DataStorePrepareGetParam, DataStoreRateObjectParam, DataStoreRatingTarget,
        DataStoreReportCourseParam, DataStoreReqGetInfo, DataStoreSearchParam,
        DataStoreUploadCourseRecordParam, GetMetaInfo, GetMetaParam, KeyValue, Permission,
        PersistenceTarget, PreparePostParam, RateCustomRankingParam, RatingInfo,
        RatingInfoWithSlot, RatingInitParamWithSlot, ReqPostInfo,
    },
};
use rnex_rmc::{qbuffer::QBuffer, qresult::QResult, response::ErrorCode, rmc_struct};
use rnex_server::PassthroughInitModule;
use rnex_util::{PID, date_time::DateTime};
use sqlx::query;
use std::convert;
use tracing::{error, info, instrument, warn};

use crate::{DatastoreManager, s3presigner::S3Presigner};
// todo: refactor this further to make some of the helper functions attached to the user and some to
// the manager and also move the usages of pid into the helper functions attached to user

#[derive(Debug)]
#[rmc_struct(DatastoreProtocol)]
pub struct DatastoreUser {
    pub base: PassthroughInitModule<BaseUser>,
    pub dm: PassthroughInitModule<DatastoreManager>,
}

impl DatastoreUser {
    #[instrument]
    fn map_row_to_meta_info(
        &self,
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
            expire_time: DateTime::PRACTICALLY_NEVER,
            created_time: DateTime::from_naive(row_creation_date),
            updated_time: DateTime::from_naive(row_update_date),
            referred_time: DateTime::from_naive(row_creation_date),
            ratings,
        }
    }

    #[instrument]
    pub async fn check_object_availability(
        &self,
        data_id: i64,
        password: i64,
    ) -> Result<(), ErrorCode> {
        let row = sqlx::query!(
            r#"
                SELECT under_review, access_password
                FROM datastore.objects
                WHERE data_id = $1 AND upload_completed = TRUE AND deleted = FALSE
                "#,
            data_id
        )
        .fetch_optional(&self.dm.db_pool)
        .await
        .map_err(|e| {
            eprintln!("Availability check DB error: {:?}", e);
            ErrorCode::DataStore_NotFound
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

    #[instrument]
    pub async fn get_object_ratings(
        &self,
        data_id: i64,
        password: i64,
    ) -> Result<Vec<RatingInfoWithSlot>, ErrorCode> {
        self.check_object_availability(data_id, password).await?;

        let rows = sqlx::query!(
            r#"
                SELECT slot, total_value, count, initial_value
                FROM datastore.object_ratings
                WHERE data_id = $1
                "#,
            data_id
        )
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            eprintln!("Ratings fetch error: {:?}", e);
            ErrorCode::DataStore_NotFound
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

    #[instrument]
    pub async fn get_object_info_by_data_id(
        &self,
        data_id: i64,
        password: i64,
    ) -> Result<GetMetaInfo, ErrorCode> {
        self.check_object_availability(data_id, password).await?;

        let row = sqlx::query!(
                r#"SELECT data_id, owner, size, name, data_type, meta_binary,
                          permission, permission_recipients, delete_permission, delete_permission_recipients,
                          period, refer_data_id, flag, tags, creation_date, update_date
                   FROM datastore.objects WHERE data_id = $1"#,
                data_id
            )
                .fetch_optional(&self.dm.db_pool)
                .await
                .map_err(|_| ErrorCode::DataStore_NotFound)?
                .ok_or(ErrorCode::DataStore_NotFound)?;

        let ratings = self.get_object_ratings(data_id, password).await?;

        // lots of ugly unwraps please fix the db eventually to correctly represent what states it can and cant be in

        Ok(self.map_row_to_meta_info(
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
            row.creation_date.unwrap_or_default(),
            row.update_date.unwrap_or_default(),
            ratings,
        ))
    }

    #[instrument]
    async fn get_object_info_by_persistence_target(
        &self,
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
                .fetch_optional(&self.dm.db_pool)
                .await
                .map_err(|_| ErrorCode::DataStore_NotFound)?
                .ok_or(ErrorCode::DataStore_NotFound)?;

        let db_password = row.access_password;
        if db_password != 0 && db_password != password {
            return Err(ErrorCode::DataStore_InvalidPassword);
        }

        if row.under_review {
            return Err(ErrorCode::DataStore_UnderReviewing);
        }

        let ratings = self.get_object_ratings(row.data_id, password).await?;

        Ok(self.map_row_to_meta_info(
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
            row.creation_date.unwrap_or_default(),
            row.update_date.unwrap_or_default(),
            ratings,
        ))
    }

    #[instrument]
    async fn get_buffer_queues_by_data_id_and_slot(
        &self,
        data_id: i64,
        slot: i32,
    ) -> Result<Vec<QBuffer>, ErrorCode> {
        self.check_object_availability(data_id, 0).await?;

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
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("Buffer queue fetch error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        let buffer_queues = rows.into_iter().map(|row| QBuffer(row.buffer)).collect();

        Ok(buffer_queues)
    }

    #[instrument]
    fn verify_object_permission(
        &self,
        owner_id: PID,
        permission: &Permission,
    ) -> Result<(), ErrorCode> {
        if owner_id == self.base.pid {
            return Ok(());
        }

        match permission.permission {
            0 => Ok(()),                                     // All can read
            1 => Err(ErrorCode::DataStore_PermissionDenied), // Friends only, unimplemented
            2 => {
                // Recipient IDs can read
                if permission.recipient_ids.contains(&self.base.pid) {
                    Ok(())
                } else {
                    Err(ErrorCode::DataStore_PermissionDenied)
                }
            }
            3 => Err(ErrorCode::DataStore_PermissionDenied), // Owner only, redundant (maple: we should still check if its
            // the owner and return true if it is, otherwise the owner would be unable to access their own objects)
            _ => Err(ErrorCode::DataStore_InvalidArgument), // ??? haxx0r
        }
    }

    #[instrument]
    fn filter_properties_by_result_option(&self, meta_info: &mut GetMetaInfo, result_option: u8) {
        if (result_option & 0x01) == 0 {
            meta_info.meta_binary = QBuffer(Vec::new());
        }

        if (result_option & 0x04) == 0 {
            meta_info.ratings = Vec::new();
        }

        // No idea what the other things do. :shrug:
    }

    #[instrument]
    async fn init_object_rating_slot(
        &self,
        data_id: i64,
        rating_param: RatingInitParamWithSlot,
    ) -> Result<(), ErrorCode> {
        info!("running init object rating slot");
        sqlx::query!(
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
        .execute(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;
        info!("done running");
    }

    // Dawg...
    #[instrument]
    async fn get_custom_rankings_by_data_ids(
        &self,
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
        .fetch_all(&self.dm.db_pool)
        .await;

        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                error!("Custom ranking query error: {:?}", e);
                return results;
            }
        };

        for row in rows {
            let data_id = row.data_id;
            let score = row.value.unwrap_or(0) as u32;

            if let Ok(meta) = self.get_object_info_by_data_id(data_id, 0).await {
                results.push(DataStoreCustomRankingResult {
                    order: 0,
                    score,
                    meta_info: meta,
                });
            } else {
                warn!("Could not find metadata for ranked object {}", data_id);
            }
        }

        results
    }

    #[instrument]
    async fn get_user_course_object_ids(&self, owner_pid: PID) -> Result<Vec<i64>, ErrorCode> {
        let rows = sqlx::query!(
            r#"
                SELECT data_id
                FROM datastore.objects
                WHERE owner = $1 AND data_type > 2 AND data_type < 50
                "#,
            owner_pid
        )
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("error fetching course IDs for PID {}: {:?}", owner_pid, e);
            ErrorCode::DataStore_NotFound
        })?;

        let mut valid_ids = Vec::new();
        for row in rows {
            let data_id = row.data_id;
            // always check avail
            if self.check_object_availability(data_id, 0).await.is_ok() {
                valid_ids.push(data_id);
            }
        }

        Ok(valid_ids)
    }

    #[instrument]
    fn get_blacklist_1(&self) -> Vec<String> {
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

    #[instrument]
    fn get_blacklist_2(&self) -> Vec<String> {
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

    #[instrument]
    fn get_blacklist_3(&self) -> Vec<String> {
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

    #[instrument]
    // couldn't find a better way to do this im going crazyy
    async fn rate_object(
        &self,
        dataid: i64,
        slot: i8,
        rating_value: i32,
        access_password: i64,
    ) -> Result<RatingInfo, ErrorCode> {
        self.check_object_availability(dataid, access_password)
            .await?;

        let rating = RatingInfo::default();

        sqlx::query!(
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
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        Ok(rating)
    }

    #[instrument]
    async fn change_meta_object_check(
        &self,
        param: &DataStoreChangeMetaParam,
    ) -> Result<(), ErrorCode> {
        let row = sqlx::query!(
                r#"
                SELECT update_password, under_review FROM datastore.objects WHERE data_id=$1 AND upload_completed=TRUE AND deleted=FALSE
                "#,
                param.dataid
            )
        .fetch_one(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        if row.update_password != 0 && row.update_password != param.update_password {
            return Err(ErrorCode::DataStore_InvalidPassword);
        }

        if row.under_review {
            return Err(ErrorCode::DataStore_UnderReviewing);
        }

        Ok(())
    }

    #[instrument]
    async fn get_rating_with_slot_data_id(
        &self,
        dataid: i64,
    ) -> Result<Vec<RatingInfoWithSlot>, ErrorCode> {
        self.check_object_availability(dataid, 0).await?;

        let rows = sqlx::query!(
        r#"
            SELECT slot, total_value, count, initial_value FROM datastore.object_ratings WHERE data_id=$1
        "#,
        dataid
    )
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
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

    #[instrument]
    pub async fn insert_buffer(&self, dataid: i64, slot: i32, buffer: &QBuffer) {
        let db_now = Utc::now().naive_utc();

        sqlx::query!(
            r#"
            INSERT INTO datastore.buffer_queues (
                data_id,
                slot,
                creation_date,
                buffer
            ) VALUES (
                $1,
                $2,
                $3,
                $4
            ) ON CONFLICT (data_id, slot, buffer) DO UPDATE SET creation_date=$3
        "#,
            dataid,
            slot,
            db_now,
            buffer.0
        )
        .execute(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        });
    }
}
impl DataStore for DatastoreUser {
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode> {
        let mut meta_info = if metaparam.dataid != 0 {
            self.get_object_info_by_data_id(metaparam.dataid, metaparam.access_password)
                .await?
        } else {
            self.get_object_info_by_persistence_target(
                metaparam.persistence_target,
                metaparam.access_password,
            )
            .await?
        };

        self.verify_object_permission(meta_info.owner, &meta_info.permission)?;

        self.filter_properties_by_result_option(&mut meta_info, metaparam.result_option);

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
        let now = Utc::now().naive_utc();

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
                        self.base.pid as i32,
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
                        now,
                        now
                    )
                    .fetch_one(&self.dm.db_pool)
                    .await
                    .map_err(|e| {
                        error!("DB Error: {:?}", e);
                        ErrorCode::DataStore_NotFound
                    })?;

        let data_id = row.data_id;

        info!("param is: {:?}", postparam);
        info!("RIP len is: {}", postparam.rating_init_params.len());
        for rating_param in &postparam.rating_init_params {
            info!("running init params");
            self.init_object_rating_slot(data_id, rating_param.clone())
                .await?;
        }

        let key = format!("data/{}.bin", data_id);

        let (upload_url, fields) = self.dm.s3_presigner.generate_presigned_post(&key).await;

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
        info!("Data ID: {:?}", completeparam.dataid);
        info!("Success: {:?}", completeparam.success);

        let record = sqlx::query!(
            r#"SELECT owner, under_review FROM datastore.objects WHERE data_id = $1"#,
            completeparam.dataid
        )
        .fetch_optional(&self.dm.db_pool)
        .await
        .map_err(|e| {
            eprintln!("select error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        let record = record.ok_or(ErrorCode::DataStore_NotFound)?;

        if record.under_review {
            return Err(ErrorCode::DataStore_UnderReviewing);
        }

        if record.owner.unwrap_or(0) as PID != self.base.pid {
            return Err(ErrorCode::DataStore_PermissionDenied);
        }

        if completeparam.success {
            sqlx::query!(
                r#"UPDATE datastore.objects SET upload_completed = true WHERE data_id = $1"#,
                completeparam.dataid
            )
            .execute(&self.dm.db_pool)
            .await
            .map_err(|e| {
                eprintln!("update error: {:?}", e);
                ErrorCode::DataStore_NotFound
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
            .fetch_one(&self.dm.db_pool)
            .await
            .map_err(|_| ErrorCode::DataStore_NotFound)?;

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
                        .execute(&self.dm.db_pool)
                        .await
                        .map_err(|e| {
                            error!("update/insert error: {:?}", e);
                            ErrorCode::DataStore_NotFound
                        })?;
        }

        Ok(())
    }

    async fn get_application_config(&self, appid: u32) -> Result<Vec<i32>, ErrorCode> {
        const MAX_COURSE_UPLOADS: i32 = 100;

        let config = match appid {
            0 => vec![
                0x0000_0001,
                0x0000_0032,
                0x0000_0096,
                0x0000_012c,
                0x0000_01f4,
                0x0000_0320,
                0x0000_0514,
                0x0000_07d0,
                0x0000_0bb8,
                0x0000_1388,
                MAX_COURSE_UPLOADS,
                0x0000_0014,
                0x0000_001e,
                0x0000_0028,
                0x0000_0032,
                0x0000_003c,
                0x0000_0046,
                0x0000_0050,
                0x0000_005a,
                0x0000_0064,
                0x0000_0023,
                0x0000_004b,
                0x0000_0023,
                0x0000_004b,
                0x0000_0032,
                0x0000_0000,
                0x0000_0003,
                0x0000_0003,
                0x0000_0064,
                0x0000_0006,
                0x0000_0001,
                0x0000_0060,
                0x0000_0005,
                0x0000_0060,
                0x0000_0000,
                0x0000_07e4,
                0x0000_0001,
                0x0000_0001,
                0x0000_000c,
                0x0000_0000,
            ],
            1 => vec![
                2,
                1_770_179_696,
                1_770_179_664,
                1_770_179_640,
                1_770_180_827,
                1_770_180_777,
                1_770_180_745,
                1_770_177_625,
                1_770_177_590,
            ],
            2 => vec![
                0x0000_07df,
                0x0000_000c,
                0x0000_0016,
                0x0000_0005,
                0x0000_0000,
            ],
            10 => vec![35, 75, 96, 40, 5, 6],
            _ => {
                error!("unknown SMM app id: {}", appid);
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

        let mut ranking_results = self
            .get_custom_rankings_by_data_ids(
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

        let buffers = self
            .get_buffer_queues_by_data_id_and_slot(bufferparam.dataid, bufferparam.slot)
            .await?;

        Ok(buffers)
    }

    async fn prepare_get_object(
        &self,
        prepare_get_param: DataStorePrepareGetParam,
    ) -> Result<DataStoreReqGetInfo, ErrorCode> {
        let meta_info = if prepare_get_param.dataid != 0 {
            info!("getting object by meta info");
            info!("Data ID: {:?}", prepare_get_param.dataid);
            self.get_object_info_by_data_id(
                prepare_get_param.dataid,
                prepare_get_param.access_password,
            )
            .await?
        } else {
            info!("getting object by persistence info");
            self.get_object_info_by_persistence_target(
                prepare_get_param.persistence_target,
                prepare_get_param.access_password,
            )
            .await?
        };

        info!("verifying object permission");
        self.verify_object_permission(meta_info.owner, &meta_info.permission)?;

        let key = format!("data/{}.bin", meta_info.dataid);
        let download_url = self.dm.s3_presigner.generate_presigned_get(&key);

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
            let course_ids = self.get_user_course_object_ids(owner_pid).await?;

            if course_ids.is_empty() {
                continue;
            }

            let mut results = self.get_custom_rankings_by_data_ids(0, course_ids).await;

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
                    res.meta_info.meta_binary = QBuffer(Vec::new());
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
            128 => self.get_blacklist_1(),
            129 => self.get_blacklist_2(),
            130 => self.get_blacklist_3(),
            _ => {
                warn!(
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
                self.get_object_info_by_data_id(param.dataid, param.access_password)
                    .await
            } else {
                self.get_object_info_by_persistence_target(
                    param.persistence_target,
                    param.access_password,
                )
                .await
            };

            match info_result {
                Ok(mut meta) => {
                    if let Err(e) = self.verify_object_permission(meta.owner, &meta.permission) {
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
                            meta.meta_binary = QBuffer(Vec::new());
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

        let now = Utc::now().naive_utc();

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
            self.base.pid as i32,
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
            now,
            now
        )
        .fetch_one(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        let data_id = row.data_id;

        for rating_param in &param.post_param.rating_init_params {
            info!("running init params");
            self.init_object_rating_slot(data_id, rating_param.clone())
                .await?;
        }

        let key = format!("data/{}.jpg", data_id);

        let (upload_url, fields) = self.dm.s3_presigner.generate_presigned_post(&key).await;

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

    async fn complete_attach_file(
        &self,
        complete_attach_param: CompletePostParam,
    ) -> Result<String, ErrorCode> {
        info!("Data ID: {:?}", complete_attach_param.dataid);
        info!("Success: {:?}", complete_attach_param.success);

        let key = format!("data/{}.jpg", complete_attach_param.dataid);
        let download_url = self.dm.s3_presigner.generate_presigned_get(&key);

        Ok(download_url)
    }

    async fn rate_objects(
        &self,
        targets: Vec<DataStoreRatingTarget>,
        params: Vec<DataStoreRateObjectParam>,
        _transactional: bool,
        fetch_ratings: bool,
    ) -> Result<(Vec<RatingInfo>, Vec<QResult>), ErrorCode> {
        let results: Vec<QResult> = vec![];

        // this might be good to keep as a sanity check but as long as we zip the two vecs together
        // we already avoid crashes which can be caused by this
        // (previous comment) SMM seems to work fine with this, no clue for other DTSR games
        // binder: we should keep it anyways, just in case. no harm no foul right?
        if targets.len() != params.len() {
            return Err(ErrorCode::DataStore_OperationNotAllowed);
        }

        let actions =
            targets
                .into_iter()
                .zip(params.into_iter())
                .map(|(target, param)| async move {
                    info!("Data ID: {:?}", target.dataid);
                    info!("Slot: {:?}", target.slot);
                    info!("Access Password: {:?}", param.access_password);

                    let object_info = self
                        .get_object_info_by_data_id(target.dataid, param.access_password)
                        .await?;
                    info!("object info get complete");
                    self.verify_object_permission(object_info.owner, &object_info.permission)?;
                    info!("object permission complete");

                    if fetch_ratings {
                        let rating = self
                            .rate_object(
                                target.dataid,
                                target.slot,
                                param.rating_value,
                                param.access_password,
                            )
                            .await?;
                        info!("rating complete");
                        Result::<Option<RatingInfo>, ErrorCode>::Ok(Some(rating))
                    } else {
                        Result::<Option<RatingInfo>, ErrorCode>::Ok(None)
                    }
                });

        let ratings: Result<Vec<_>, ErrorCode> = join_all(actions).await.into_iter().collect();
        let ratings = ratings?;

        let ratings = if fetch_ratings {
            ratings.into_iter().filter_map(convert::identity).collect()
        } else {
            // skip collecting, we already know the vector is empty
            vec![]
        };

        Ok((ratings, results))
    }

    async fn change_meta(&self, param: DataStoreChangeMetaParam) -> Result<(), ErrorCode> {
        let object_info = self.get_object_info_by_data_id(param.dataid, 0).await?;
        self.verify_object_permission(object_info.owner, &object_info.permission)?;

        if param.modifies_flag & 0x08 != 0 {
            self.change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET period=$1 WHERE data_id=$2"#,
                param.period as i16,
                param.dataid
            )
            .execute(&self.dm.db_pool)
            .await
            .map_err(|e| {
                eprintln!("update error: {:?}", e);
                ErrorCode::DataStore_NotFound
            })?;
        }

        if param.modifies_flag & 0x10 != 0 {
            self.change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET meta_binary=$1 WHERE data_id=$2"#,
                param.meta_binary.0,
                param.dataid
            )
            .execute(&self.dm.db_pool)
            .await
            .map_err(|e| {
                eprintln!("update error: {:?}", e);
                ErrorCode::DataStore_NotFound
            })?;
        }

        if param.modifies_flag & 0x80 != 0 {
            self.change_meta_object_check(&param).await?;

            sqlx::query!(
                r#"UPDATE datastore.objects SET data_type=$1 WHERE data_id=$2"#,
                param.data_type as i16,
                param.dataid
            )
            .execute(&self.dm.db_pool)
            .await
            .map_err(|e| {
                eprintln!("update error: {:?}", e);
                ErrorCode::DataStore_NotFound
            })?;
        }

        Ok(())
    }

    async fn recommended_course_search_object(
        &self,
        _course_search_param: DataStoreSearchParam,
        _extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode> {
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
            FROM (
                SELECT * FROM datastore.objects object
                WHERE
                    object.upload_completed = TRUE AND
                    object.deleted = FALSE AND
                    object.under_review = FALSE
            ) object
            JOIN (
                SELECT data_id, value
                FROM datastore.object_custom_rankings ranking
                WHERE ranking.application_id = 0
            ) ranking
            ON
                object.data_id = ranking.data_id
            ORDER BY RANDOM()
            LIMIT 100
        "#
        )
        .fetch(&self.dm.db_pool);

        while let Some(row) = stream.try_next().await.map_err(|e| {
            eprintln!("stream error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })? {
            let permission = Permission {
                permission: row.permission.unwrap_or(0) as u8,
                recipient_ids: row.permission_recipients.unwrap_or_default(),
            };

            let del_permission = Permission {
                permission: row.delete_permission.unwrap_or(0) as u8,
                recipient_ids: row.delete_permission_recipients.unwrap_or_default(),
            };

            let meta_binary = row.meta_binary.map(QBuffer).unwrap_or_default();

            let created_time = row
                .creation_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let updated_time = row
                .update_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let referred_time = row
                .creation_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let meta_info = GetMetaInfo {
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
                expire_time: DateTime::PRACTICALLY_NEVER,
                created_time,
                updated_time,
                referred_time,
                ratings: self.get_rating_with_slot_data_id(row.data_id).await?,
            };

            let course = DataStoreCustomRankingResult {
                order: 0,
                score: row.value.unwrap_or(0) as u32,
                meta_info,
            };

            courses.push(course);
        }

        Ok(courses)
    }

    async fn upload_course_record(
        &self,
        upload_course_record_param: DataStoreUploadCourseRecordParam,
    ) -> Result<(), ErrorCode> {
        let now = Utc::now().naive_utc();

        sqlx::query!(
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
            self.base.pid,
            self.base.pid,
            upload_course_record_param.score,
            now,
            now
        )
            .execute(&self.dm.db_pool)
            .await
            .map_err(|e| {
                error!("DB Error: {:?}", e);
                ErrorCode::DataStore_NotFound
            })?;

        Ok(())
    }

    async fn get_course_record(
        &self,
        get_course_record_param: DataStoreGetCourseRecordParam,
    ) -> Result<DataStoreGetCourseRecordResult, ErrorCode> {
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
        .fetch_one(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        Ok(DataStoreGetCourseRecordResult {
            dataid: get_course_record_param.dataid,
            slot: get_course_record_param.slot,
            first_pid: row.first_pid as u32,
            best_pid: row.best_pid as u32,
            best_score: row.best_score,
            created_time: DateTime::PRACTICALLY_NEVER,
            updated_time: DateTime::PRACTICALLY_NEVER,
        })
    }

    async fn add_to_buffer_queues(
        &self,
        bufferparam: Vec<BufferQueueParam>,
        buffers: Vec<QBuffer>,
    ) -> Result<Vec<QResult>, ErrorCode> {
        let mut results = Vec::new();

        let client_pid = self.base.pid;

        for (param, buffer) in bufferparam.iter().zip(buffers.iter()) {
            if param.slot == 0 {
                let object_info = self.get_object_info_by_data_id(param.dataid, 0).await?;

                if object_info.data_type == 1 && object_info.owner != client_pid {
                    return Err(ErrorCode::DataStore_PermissionDenied);
                }
            }

            self.insert_buffer(param.dataid, param.slot, buffer).await;

            results.push(QResult::success(ErrorCode::Core_Unknown));
        }

        Ok(results)
    }

    async fn get_object_infos(
        &self,
        dataids: Vec<i64>,
    ) -> Result<Vec<DataStoreFileServerObjectInfo>, ErrorCode> {
        let mut list = Vec::with_capacity(dataids.len());
        for dataid in dataids.into_iter() {
            let object_info = self.get_object_info_by_data_id(dataid, 0).await?;

            let key = format!("data/{}.bin", dataid);
            let download_url = self.dm.s3_presigner.generate_presigned_get(&key);

            list.push(DataStoreFileServerObjectInfo {
                dataid,
                get_info: DataStoreReqGetInfo {
                    url: download_url,
                    request_headers: vec![],
                    size: object_info.size,
                    root_ca_cert: vec![],
                    dataid,
                },
            });
        }

        Ok(list)
    }

    async fn check_rate_custom_ranking_counter(
        &self,
        application_id: u32,
    ) -> Result<bool, ErrorCode> {
        if application_id == 0 {
            Ok(true)
        } else {
            Err(ErrorCode::Core_Unknown)
        }
    }

    async fn report_course(
        &self,
        report_course_param: DataStoreReportCourseParam,
    ) -> Result<(), ErrorCode> {
        sqlx::query!(
            r#"
                INSERT INTO datastore.reports (
                    data_id,
                    reporter_pid,
                    category,
                    reason
                ) VALUES (
                    $1, $2, $3, $4
                )
            "#,
            report_course_param.dataid,
            self.base.pid,
            report_course_param.report_category as i16,
            report_course_param.report_reason
        )
        .execute(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::Core_NotImplemented // i don't know why, but returning this makes the game show "Report sent OK" so i'll use it
        })?;

        Ok(())
    }

    async fn delete_object(&self, param: DataStoreDeleteParam) -> Result<(), ErrorCode> {
        let row = sqlx::query!(
            r#"
                SELECT update_password
                FROM datastore.objects
                WHERE data_id = $1 AND upload_completed = TRUE AND deleted = FALSE
            "#,
            param.dataid
        )
        .fetch_one(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        let passwd = row.update_password;

        if param.update_password != passwd {
            return Err(ErrorCode::DataStore_PermissionDenied);
        }

        info!("update password check passed");

        sqlx::query!(
            "UPDATE datastore.objects SET deleted=true WHERE data_id=$1",
            param.dataid
        )
        .execute(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        Ok(())
    }

    async fn get_custom_ranking(
        &self,
        param: DataStoreGetCustomRankingParam,
    ) -> Result<(Vec<DataStoreCustomRankingResult>, Vec<QResult>), ErrorCode> {
        let mut ranking_results = Vec::new();

        let rows = query!(
            r#"
            SELECT
                data_id,
                value
            FROM datastore.object_custom_rankings
            WHERE application_id = $1
              AND value >= $2
              AND value <= $3
            ORDER BY value DESC
            LIMIT $4 OFFSET $5
            "#,
            param.application_id as i64,
            param.condition.min_value as i64,
            param.condition.max_value as i64,
            param.result_range.size as i64,
            param.result_range.offset as i64,
        )
        .fetch_all(&self.dm.db_pool)
        .await
        .map_err(|e| {
            error!("DB Error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })?;

        let mut current_order = param.result_range.offset + 1;

        for row in rows {
            let data_id = row.data_id;
            let score = row.value.unwrap_or(0) as u32;

            if let Ok(meta) = self.get_object_info_by_data_id(data_id, 0).await {
                ranking_results.push(DataStoreCustomRankingResult {
                    order: current_order,
                    score,
                    meta_info: meta,
                });
            } else {
                warn!("could not find metadata for ranked object {}", data_id);
            }

            current_order += 1;
        }

        let mut q_results = Vec::with_capacity(ranking_results.len());

        for result in &mut ranking_results {
            if (param.result_option & 0x01) == 0 {
                result.meta_info.tags = Vec::new();
            }

            if (param.result_option & 0x02) == 0 {
                result.meta_info.ratings = Vec::new();
            }

            if (param.result_option & 0x04) == 0 {
                result.meta_info.meta_binary = QBuffer(Vec::new());
            }

            if (param.result_option & 0x20) == 0 {
                result.score = 0;
            }

            q_results.push(QResult::success(ErrorCode::Core_Unknown));
        }

        Ok((ranking_results, q_results))
    }

    // todo: respect the search parameters and extra data
    async fn ctr_pickup_course_search_object(
        &self,
        _course_search_param: DataStoreSearchParam,
        _extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode> {
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
            FROM (
                SELECT * FROM datastore.objects object
                WHERE
                    object.upload_completed = TRUE AND
                    object.deleted = FALSE AND
                    object.under_review = FALSE
            ) object
            JOIN (
                SELECT data_id, value
                FROM datastore.object_custom_rankings ranking
                WHERE ranking.application_id = 0
            ) ranking
            ON
                object.data_id = ranking.data_id
            ORDER BY RANDOM()
            LIMIT 100
        "#
        )
        .fetch(&self.dm.db_pool);

        while let Some(row) = stream.try_next().await.map_err(|e| {
            eprintln!("stream error: {:?}", e);
            ErrorCode::DataStore_NotFound
        })? {
            let permission = Permission {
                permission: row.permission.unwrap_or(0) as u8,
                recipient_ids: row.permission_recipients.unwrap_or_default(),
            };

            let del_permission = Permission {
                permission: row.delete_permission.unwrap_or(0) as u8,
                recipient_ids: row.delete_permission_recipients.unwrap_or_default(),
            };

            let meta_binary = row.meta_binary.map(QBuffer).unwrap_or_default();

            let created_time = row
                .creation_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let updated_time = row
                .update_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let referred_time = row
                .creation_date
                .map(DateTime::from_naive)
                .unwrap_or_default();

            let meta_info = GetMetaInfo {
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
                expire_time: DateTime::PRACTICALLY_NEVER,
                created_time,
                updated_time,
                referred_time,
                ratings: self.get_rating_with_slot_data_id(row.data_id).await?,
            };

            let course = DataStoreCustomRankingResult {
                order: 0,
                score: row.value.unwrap_or(0) as u32,
                meta_info,
            };

            courses.push(course);
        }

        Ok(courses)
    }
}
