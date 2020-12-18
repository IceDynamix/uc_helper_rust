use mongodb::{Client, Database};
use serenity::static_assertions::_core::fmt::Formatter;
use std::env;

const DATABASE: &str = "uc_helper";

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionFailed,
    NotFound,
    CouldNotPush,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            DatabaseError::ConnectionFailed => f.write_str("ConnectionFailed"),
            DatabaseError::NotFound => f.write_str("NotFound"),
            DatabaseError::CouldNotPush => f.write_str("CouldNotPush"),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn description(&self) -> &str {
        match *self {
            DatabaseError::ConnectionFailed => "Connection to database failed",
            DatabaseError::NotFound => "Could not find item",
            DatabaseError::CouldNotPush => "Could not push to database",
        }
    }
}

async fn establish_db_connection() -> Result<Database, DatabaseError> {
    let url = env::var("DATABASE_URL").expect("url must be set");
    match Client::with_uri_str(&url).await {
        Ok(client) => Ok(client.database(DATABASE)),
        Err(_) => Err(DatabaseError::ConnectionFailed),
    }
}

pub mod players {
    use crate::tetrio::database::{establish_db_connection, DatabaseError};
    use crate::tetrio::{tenchi, Rank, User};
    use chrono::SecondsFormat;
    use mongodb::bson;
    use mongodb::bson::doc;
    use mongodb::options::FindOneAndReplaceOptions;
    use serde::{Deserialize, Serialize};
    use tokio::stream::StreamExt;

    const COLLECTION: &str = "players";

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PlayerEntry {
        _id: String,
        username: String,
        data: User,
        highest_rank: String,
        timestamp: String,
    }

    pub async fn get_cached(username: &str) -> Result<Option<PlayerEntry>, DatabaseError> {
        let collection = establish_db_connection().await?.collection(COLLECTION);

        let mut results = collection
            .find(
                doc! {"$or": [{"_id": username}, {"username": username}]},
                None,
            )
            .await
            .unwrap();

        match results.next().await {
            Some(result) => match result {
                Ok(r) => Ok(Some(bson::from_document(r).unwrap())),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub async fn get(username: &str) -> Result<PlayerEntry, DatabaseError> {
        let cached = get_cached(username).await?;

        let now = chrono::offset::Utc::now();
        if let Some(cached) = cached {
            let last_update = chrono::DateTime::parse_from_rfc3339(&cached.timestamp).unwrap();
            if now - chrono::Duration::minutes(10) < last_update {
                return Ok(cached);
            }
        }

        let data = match crate::tetrio::User::request(username).await {
            Some(data) => data,
            None => return Err(DatabaseError::NotFound),
        };

        // for our purposes its ok if it fails
        let highest_ranks = tenchi::HighestRanks::from_cache().ok();
        let highest_rank = match highest_ranks {
            Some(history) => history.get(username),
            None => Rank::Unranked,
        }
        .to_str()
        .to_string();

        let timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
        let _id = data.clone()._id;
        let username = data.clone().username;

        let entry = PlayerEntry {
            _id: _id.clone(),
            username,
            data,
            highest_rank,
            timestamp,
        };
        let bson_entry = bson::to_document(&entry).unwrap();

        let collection = establish_db_connection().await?.collection(COLLECTION);
        let options = FindOneAndReplaceOptions::builder().upsert(true).build();
        match collection
            .find_one_and_replace(doc! {"_id": _id.clone()}, bson_entry, options)
            .await
        {
            Ok(_) => Ok(entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }
}

pub mod discord {
    use crate::tetrio::database::{establish_db_connection, DatabaseError};
    use chrono::SecondsFormat;
    use mongodb::bson;
    use mongodb::bson::doc;
    use mongodb::options::FindOneAndReplaceOptions;
    use serde::{Deserialize, Serialize};

    const COLLECTION: &str = "discord";

    #[derive(Serialize, Deserialize, Debug)]
    pub struct DiscordEntry {
        discord_id: u64,
        tetrio_id: String,
        timestamp: String,
    }

    pub async fn link(discord_id: u64, tetrio_id: &str) -> Result<DiscordEntry, DatabaseError> {
        let now = chrono::offset::Utc::now();
        let timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);

        let entry = DiscordEntry {
            discord_id,
            tetrio_id: tetrio_id.to_string(),
            timestamp,
        };
        let bson_entry = bson::to_document(&entry).unwrap();

        let collection = establish_db_connection().await?.collection(COLLECTION);
        let options = FindOneAndReplaceOptions::builder().upsert(true).build();
        match collection
            .find_one_and_replace(doc! {"discord_id": discord_id}, bson_entry, options)
            .await
        {
            Ok(_) => Ok(entry),
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }

    pub async fn unlink(discord_id: &str) -> Result<(), DatabaseError> {
        let collection = establish_db_connection().await?.collection(COLLECTION);
        match collection
            .find_one_and_delete(doc! {"discord_id": discord_id}, None)
            .await
        {
            Ok(result) => match result {
                Some(_) => Ok(()),
                None => Err(DatabaseError::NotFound),
            },
            Err(_) => Err(DatabaseError::CouldNotPush),
        }
    }
}
