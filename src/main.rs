use std::env;
use std::fs;
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

const RUN_INTERNAL: &str = "__r_u_n__";

enum Command {
    Exit(i32),
    Echo(String),
    Type(String),
    Run(String)
}

fn find_system_command_path(command_name: &str) -> Result<Option<String>, String> {
    match env::var("PATH") {
        Ok(value) => {
            let directories = value.split(":");
            for directory in directories {
                let dir_entries =
                    fs::read_dir(directory).expect(&format!("failed to read dir: {}", directory));
                for dir_entry in dir_entries {
                    let full_path =
                        dir_entry.expect(&format!("failed to read file in dir: {}", directory));
                    let filename = full_path.file_name().into_string().expect(&format!(
                        "failed to read file: {}",
                        full_path.path().display()
                    ));
                    if filename == command_name {
                        match full_path.path().to_str() {
                            Some(path) => return Ok(Some(String::from(path))),
                            None => {
                                return Err(format!(
                                    "failed to get full path to system folder of {} command",
                                    filename
                                ));
                            }
                        }
                    }
                }
            }

            return Ok(None);
        }
        Err(_e) => {
            return Err(String::from(
                "failed to get PATH variable to find commands in system folders",
            ));
        }
    }
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
                // try to find this command in user system folders
                match find_system_command_path(typed_command_name) {
                    Ok(Some(path)) => {
                        return Ok(Command::Type(format!(
                            "{} is {}",
                            String::from(typed_command_name),
                            String::from(path)
                        )));
                    }
                    Ok(None) => Ok(Command::Type(format!(
                        "{}: not found",
                        String::from(typed_command_name)
                    ))),
                    Err(_e) => {
                        return Err(String::from(
                            "failed to get PATH variable to find commands in system folders",
                        ));
                    }
                }
            }
        }),
    );

    // internal command, not for using from shell
    command_env.push(
        String::from(RUN_INTERNAL),
        Box::new(|command_tokens, _| {
            let command_name = command_tokens[0].trim();
            match find_system_command_path(command_name) {
                Ok(Some(path)) => {
                    let args = &command_tokens[1..];

                    let result = process::Command::new(path)
                    .args(args)
                    .output();

                    match result {
                        Ok(output) => {
                            if output.status.success() {
                                return Ok(Command::Run(String::from_utf8(output.stdout).expect("failed to read from program stdout")));
                            } else {
                                return Ok(Command::Run(String::from_utf8(output.stderr).expect("failed to read from program stderr")));
                            }
                        }
                        Err(err) => {
                            return Err(format!(
                                "failed to execute program: {}", err
                            ));
                        }
                    }
                }
                Ok(None) => Ok(Command::Run(format!(
                    "{}: not found",
                    String::from(command_name)
                ))),
                Err(_e) => {
                    return Err(String::from(
                        "failed to get PATH variable to find commands in system folders",
                    ));
                }
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
        match command_env
            .0
            .iter()
            .find(|(command_name, _)| command_name == command_tokens[0])
        {
            Some(command_object) => return command_object.1(&command_tokens, command_env),
            None => {
                // try to run find command in system folder (using PATH) and run it
                let command_run = command_env.0.iter().find(|(command_name, _)| command_name == RUN_INTERNAL).unwrap();
                return command_run.1(&command_tokens, command_env);
            }
        }
    } else {
        return Err(String::from("command not specified"));
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = String::new();
    let command_env = init();

    loop {
        print_invite_symb();
        stdin.read_line(&mut input).unwrap();

        match handle_input(&input, &command_env) {
            Ok(command) => match command {
                Command::Exit(code) => process::exit(code),
                Command::Echo(output) => println!("{}", output.trim()),
                Command::Type(command) | Command::Run(command) => println!("{}", command),
            },
            Err(desc) => println!("{}", desc),
        }

        input.clear();
    }
}
