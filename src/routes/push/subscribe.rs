use crate::database::*;
use crate::util::result::{Error, Result};

use mongodb::bson::{doc, to_document};
use rauth::auth::Session;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Subscription {
    endpoint: String,
    p256dh: String,
    auth: String,
}

#[post("/subscribe", data = "<data>")]
pub async fn req(session: Session, data: Json<Subscription>) -> Result<()> {
    let data = data.into_inner();
    let col = get_collection("accounts")
        .update_one(
            doc! {
                "_id": session.user_id,
                "sessions.id": session.id.unwrap()
            },
            doc! {
                "$set": {
                    "sessions.$.subscription": to_document(&data).unwrap()
                }
            },
            None,
        )
        .await
        .unwrap();

    Ok(())
}
