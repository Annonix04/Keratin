use std::{
    path::Path,
    process::{
        Command,
        exit,
    },
    io::{
        self,
        Write
    },
    env::{
        self,
        var
    },
};

const PROMPT: &str = "#~ ";

fn main() {
    loop {
        let raw_line = get_command();
        let line = raw_line.split(" ").collect::<Vec<&str>>();

        let cmd = line.get(0).unwrap().to_owned();
        let params = if line.len() > 1 { line[1..].to_vec() } else { Vec::new() };

        process_command(cmd, params);

        io::stdout().flush().unwrap();
    }
}

fn get_command() -> String {
    print!("{}", PROMPT);
    io::stdout().flush().unwrap();

    let input = io::stdin();
    let mut command = String::new();
    input.read_line(&mut command).unwrap();

    io::stdout().flush().unwrap();

    command.trim().to_string()
}

fn process_command(cmd: &str, params: Vec<&str>) {
    let raw_path = var("PATH").unwrap();
    let path = env::split_paths(&raw_path)
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<String>>();

    let builtins = vec![
        "exit",
        "echo",
        "path",
        "type",
        "exec",
    ];

    match cmd {
        "exit" => {
            io::stdout().flush().unwrap();

            if params.is_empty() {
                exit(0);
            }

            exit(params[0].parse::<i32>().unwrap());
        },
        "echo" => {
            let arg = params.join(" ");

            println!("{arg}");
        },
        "path" => {
            println!("{path:?}");
        },
        "type" => {
            let arg = params.get(0).unwrap();

            match arg.to_owned() {
                _ if builtins.contains(arg) => {
                    println!("{arg} is a shell builtin");
                },
                _ => {
                    let exec = search_for_exec(arg, path);

                    if let Some(file) = exec {
                        println!("{arg} is {file}");
                    } else {
                        eprintln!("{arg}: not found");
                    }
                }
            }
        },
        "exec" => {
            let exec_arg = params.get(0).unwrap().to_owned();
            let exec = search_for_exec(exec_arg, path);

            match exec {
                Some(val) => {
                    let mut command = Command::new(val);

                    for param in params[1..].iter() {
                        command.arg(param);
                    }
                    let child = command.spawn().unwrap();
                    println!("\n");
                    child.wait_with_output().unwrap();
                },
                None => {
                    eprintln!("{cmd}: command not found");
                }
            }
        },
        _ => {
            eprintln!("{cmd}: not a shell command");
        },
    }
}

fn search_for_exec(cmd: &str, paths: Vec<String>) -> Option<String> {
    for folder in paths {
        let arg_path;

        if env::consts::OS == "windows" {
            arg_path = if cmd.ends_with(".exe") {
                format!("{folder}\\{cmd}")
            } else {
                format!("{folder}\\{cmd}.exe")
            };
        } else {
            arg_path = format!("{folder}/{cmd}");
        }

        let res_path = Path::new(&arg_path);

        if res_path.exists() &&
            !res_path.metadata().unwrap().permissions().readonly() {
            return Some(arg_path);
        }
    }
    None
}