#![allow(missing_docs)]
use std::str::FromStr;

use irc::proto::message::Tag;

pub trait TwitchMessageExt {
    type Error: std::error::Error;
    fn is_send_by_mod(&self) -> bool;
    fn message_id(&self) -> Option<&str>;
    fn user_id(&self) -> Option<&str>;
    fn display_name(&self) -> Option<&str>;
    fn badges(&self) -> Option<Vec<Badge>>;
    fn set_reply_parent_id(&mut self, id: &str);
    fn set_reply_parent(&mut self, parent_message: &Self) {
        if let Some(id) = parent_message.message_id() {
            self.set_reply_parent_id(id);
        }
    }
    /// Create a new reply message for the given message
    fn new_reply(&self, message: &str) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

#[derive(thiserror::Error, Debug)]
pub enum MessageError {
    #[error("Invalid message command")]
    InvalidCommand,
    #[error("Message id required")]
    MissingId,
}

pub struct Badge {
    pub badge: String,
    pub version: String,
}

impl FromStr for Badge {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, '/');
        let badge = split.next().ok_or(())?.to_owned();
        let version = split.next().ok_or(())?.to_owned();
        Ok(Self { badge, version })
    }
}

impl TwitchMessageExt for irc::proto::Message {
    type Error = MessageError;

    fn is_send_by_mod(&self) -> bool {
        let Some(tags) = &self.tags else {
            return false;
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

    fn badges(&self) -> Option<Vec<Badge>> {
        let tags = self.tags.as_ref()?;

        let badges = match tags.iter().find(|Tag(key, _)| key == "badges") {
            Some(Tag(_, Some(badges))) => badges,
            _ => return None,
        };

        let badges = badges
            .split(',')
            .map(|s| s.parse().ok())
            .collect::<Option<Vec<_>>>();

        badges
    }

    fn display_name(&self) -> Option<&str> {
        self.tags
            .as_ref()?
            .iter()
            .find_map(|t| (t.0 == "display-name").then_some(t.1.as_deref()))?
    }

    fn message_id(&self) -> Option<&str> {
        self.tags
            .as_ref()?
            .iter()
            .find_map(|t| (t.0 == "id").then_some(t.1.as_deref()))?
    }

    fn user_id(&self) -> Option<&str> {
        self.tags
            .as_ref()?
            .iter()
            .find_map(|t| (t.0 == "user-id").then_some(t.1.as_deref()))?
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
        use irc::proto::{Command::PRIVMSG, Message};
        let PRIVMSG(channel, _) = &self.command else {
            return Err(MessageError::InvalidCommand);
        };

        let tags = match self.message_id() {
            Some(id) => Some(vec![Tag(
                "reply-parent-msg-id".to_owned(),
                Some(id.to_owned()),
            )]),
            None => return Err(MessageError::MissingId),
        };

        let reply = Message {
            prefix: None,
            command: PRIVMSG(channel.to_owned(), message.to_owned()),
            tags,
        };

        Ok(reply)
    }
}
