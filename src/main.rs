/// An autonomous agent identified by name and carrying a message.
pub struct Agent {
    /// Display name for this agent.
    pub name: String,
    /// Message associated with this agent.
    pub message: String,
}

fn main() {
    let agent = Agent {
        name: String::from("Aria"),
        message: String::from("Hello, world!"),
    };
    println!("{}", agent.message);
}

#[cfg(test)]
mod tests {
    use super::Agent;

    #[test]
    fn agent_holds_name_and_message() {
        let agent = Agent {
            name: String::from("test"),
            message: String::from("ping"),
        };
        assert_eq!(agent.name, "test");
        assert_eq!(agent.message, "ping");
    }
}
