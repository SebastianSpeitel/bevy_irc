use irc::proto::message::Tag;
use thiserror::Error;

pub trait TwitchMessageExt {
    type Error: std::error::Error;
    fn is_send_by_mod(&self) -> bool;
    fn message_id(&self) -> Option<&str>;
    fn display_name(&self) -> Option<&str>;
    fn set_reply_parent_id(&mut self, id: &str);
    fn set_reply_parent(&mut self, parent_message: &Self) {
        if let Some(id) = parent_message.message_id() {
            self.set_reply_parent_id(id);
        }
    }
    fn new_reply(&self, message: &str) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[derive(Error, Debug)]
pub enum MessageError {
    #[error("Invalid message command")]
    InvalidCommand,
    #[error("Message id required")]
    MissingId,
}

impl TwitchMessageExt for super::Message {
    type Error = MessageError;

    fn is_send_by_mod(&self) -> bool {
        let tags = match &self.tags {
            Some(tags) => tags,
            None => return false,
        };

        for Tag(key, val) in tags {
            match (key.as_str(), val) {
                ("mod", Some(v)) if v == "1" => return true,
                ("user-type", Some(v)) if v == "mod" => return true,
                ("badges", Some(v)) if v.contains("moderator") => return true,
                ("badges", Some(v)) if v.contains("broadcaster") => return true,
                _ => (),
            }
        }
        false
    }

    fn display_name(&self) -> Option<&str> {
        let tags = match &self.tags {
            Some(tags) => tags,
            None => return None,
        };

        for Tag(key, val) in tags {
            if key == "display-name" {
                return val.as_deref();
            }
        }
        None
    }

    fn message_id(&self) -> Option<&str> {
        let tags = match &self.tags {
            Some(tags) => tags,
            None => return None,
        };

        for Tag(key, val) in tags {
            if key == "id" {
                return val.as_deref();
            }
        }
        None
    }

    fn set_reply_parent_id(&mut self, id: &str) {
        let tag = Tag("reply-parent-msg-id".to_owned(), Some(id.to_owned()));

        match &mut self.tags {
            Some(tags) => {
                tags.push(tag);
            }
            None => {
                self.tags = Some(vec![tag]);
            }
        }
    }

    fn new_reply(&self, message: &str) -> Result<Self, Self::Error> {
        use irc::proto::Command::PRIVMSG;
        let channel = match &self.command {
            PRIVMSG(ref channel, _) => channel,
            _ => return Err(MessageError::InvalidCommand),
        };

        let tags = match self.message_id() {
            Some(id) => Some(vec![Tag(
                "reply-parent-msg-id".to_owned(),
                Some(id.to_owned()),
            )]),
            None => return Err(MessageError::MissingId),
        };

        Ok(Self {
            prefix: None,
            command: PRIVMSG(channel.to_owned(), message.to_owned()),
            tags,
        })
    }
}
