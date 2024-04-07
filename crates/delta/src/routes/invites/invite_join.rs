use futures::future::join_all;
use revolt_database::{util::reference::Reference, Channel, Database, Invite, Member, User};
use revolt_models::v0::{self, InviteJoinResponse};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Join Invite
///
/// Join an invite by its ID
#[openapi(tag = "Invites")]
#[post("/<target>")]
pub async fn join(
    db: &State<Database>,
    user: User,
    target: Reference,
) -> Result<Json<v0::InviteJoinResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    user.can_acquire_server(db).await?;

    let invite = target.as_invite(db).await?;
    match &invite {
        Invite::Server { server, .. } => {
            let server = db.fetch_server(server).await?;
            let channels = Member::create(db, &server, &user, None).await?;
            Ok(Json(InviteJoinResponse::Server {
                channels: channels.into_iter().map(|c| c.into()).collect(),
                server: server.into(),
            }))
        }
        Invite::Group {
            channel, creator, ..
        } => {
            let mut channel = db.fetch_channel(channel).await?;
            channel.add_user_to_group(db, &user, creator).await?;
            if let Channel::Group { recipients, .. } = &channel {
                Ok(Json(InviteJoinResponse::Group {
                    users: join_all(
                        db.fetch_users(&recipients)
                            .await?
                            .into_iter()
                            .map(|other_user| other_user.into_known(&user)),
                    )
                    .await,
                    channel: channel.into(),
                }))
            } else {
                unreachable!()
            }
        }
    }
}
