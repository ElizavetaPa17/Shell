#[allow(unused_imports)]
use std::io::{self, Write};
use std::process;

type CommandFn<C> = Box<dyn Fn(&[&str], &C) -> Result<Command, String>>;
struct CommandEnv(Vec<(String, CommandFn<Self>)>);

impl CommandEnv {
    fn push(&mut self, name: String, cmdfn: CommandFn<Self>) {
        self.0.push((name, cmdfn));
    }

    fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| &name[..])
    }
}

enum Command {
    Exit(i32),
    Echo(String),
    Type(String),
}

fn init() -> CommandEnv {
    let mut command_env = CommandEnv(vec![]);

    // the first token in command_tokens is always a command name
    command_env.push(
        String::from("exit"),
        Box::new(|command_tokens, _| {
            if command_tokens.len() == 2 {
                match command_tokens[1].trim().parse() {
                    Ok(code) => Ok(Command::Exit(code)),
                    Err(_) => Err(String::from("invalid error code")),
                }
            } else {
                Err(String::from("invalid exit command: exit <error_code>"))
            }
        }),
    );

    command_env.push(
        String::from("echo"),
        Box::new(|command_tokens, _| {
            let input: String = command_tokens.join(" ");
            match input.strip_prefix(command_tokens[0]) {
                Some(output) => Ok(Command::Echo(String::from(output))),
                None => Err(String::from("invalid echo command: echo <string>")),
            }
        }),
    );

    command_env.push(
        String::from("type"),
        Box::new(|command_tokens: &[&str], command_env| {
            if command_tokens.len() != 2 {
                return Err(String::from("invalid type command: type <command>"));
            }

            let typed_command_name = command_tokens[1].trim();
            if command_env.names().any(|name| name == typed_command_name) {
                Ok(Command::Type(format!(
                    "{} is a shell builtin",
                    String::from(typed_command_name)
                )))
            } else {
                Ok(Command::Type(format!(
                    "{} not found",
                    String::from(typed_command_name)
                )))
            }
        }),
    );

    command_env
}

fn print_invite_symb() {
    print!("$ ");
    io::stdout().flush().unwrap();
}

fn handle_input(input: &str, command_env: &CommandEnv) -> Result<Command, String> {
    let command_tokens: Vec<&str> = input.split(" ").collect();

    if command_tokens.len() > 0 {
        match command_env.0.iter().find(|(command_name, _)| command_name == command_tokens[0]) {
            Some(command_object) => return command_object.1(&command_tokens, command_env),
            None => return Err(format!("{}: command not found", command_tokens[0]))
        }
    } else {
        return Err(String::from("command not specified"));
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = String::new();
    let mut command_env = init();

    loop {
        print_invite_symb();
        stdin.read_line(&mut input).unwrap();

        match handle_input(&input, &command_env) {
            Ok(command) => {
                match command {
                    Command::Exit(code) => process::exit(code),
                    Command::Echo(output) => println!("{}", output.trim()),
                    Command::Type(command) => println!("{}", command)
                }
            },
            Err(desc) => println!("{}", desc)
        }

        input.clear();
    };
}
