use std::fs;
use std::io::Write;
use clap::{Parser, Subcommand};
use inquire::{InquireError, Select};
use crate::config::{Config, create_default_config};
use crate::setup::setup;

mod os_setup;
mod outputs;
mod setup;
mod tools;
mod config;
mod tests;

#[derive(Parser)]
#[clap(name = "jarvy", version = "1.0", author = "Zac Clifton")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up the environment based on the configuration file
    Setup {
        /// Path to the configuration file
        #[clap(short, long, default_value = "./jarvy.toml")]
        file: String,
    },
    Configure {}
}

fn main() {

    // check jarvy config for the usr
    let home_dir = dirs::home_dir().expect("Failed to get home directory");

    // Create the .jarvy directory path
    let jarvy_dir = home_dir.join(".jarvy");

    // Create the .jarvy directory if it doesn't exist
    if !jarvy_dir.exists() {
        fs::create_dir(&jarvy_dir).expect("Unable to create jarvy config file");
        println!(r"
Jarvy tool collects telemetry data to help us improve your experience.
The data collected is anonymized and used solely for analytics purposes.
If you wish to opt-out of telemetry collection, you can disable it by adding the following line to your configuration file located at ~/.jarvy/config.toml:
[settings]
TELEMETRY = false

Thank you for using Jarvy!
        ");

        // Define the path to the config.toml file
        let config_file_path = jarvy_dir.join("config.toml");

        // Sample configuration content
        let config_content = r#"
        [settings]
        "#;

        // Write the content to the config.toml file
        let mut file = fs::File::create(config_file_path).expect("Unable to create config file");
        file.write_all(config_content.as_bytes()).expect("Unable to write content to config file");

    }


    // Run the CLI Parser and commands
    let cli = Cli::parse();

    match &cli.command {
        Commands::Setup { file } => {
            let config = Config::new(file);
            let tools = config.get_tool_configs();

            for (id, tool) in tools {
                println!(
                    "Installing {}: {} version {} using package manager: {}",
                    id,tool.name, tool.version, tool.version_manager
                );
                // Call the appropriate installer function here
            }
        }
        Commands::Configure { } => {
            create_default_config()
        }
    }
    print_logo();

    println!(
        "\t\tHi, I'm Jarvy by Autoscaler!
Welcome to the codebase of the open-source autoscaling infrastructure for all!"
    );

}

fn user_select() {
    let options = vec![
        "Run the project",
        "Test the project",
        "Development environment setup",
    ];

    let selection: Result<&str, InquireError> =
        Select::new("What would you like to do today?", options).prompt();

    match selection {
        Ok(choice) => {
            println!("selection: {}", choice);
            match choice {
                "Run the project" => {
                    println!("R");
                    match std::process::Command::new("cargo").arg("run").output() {
                        Ok(output) => {
                            // Handle the output here
                            println!("Output: {}", String::from_utf8_lossy(&output.stdout));
                        }
                        Err(e) => println!("Failed to execute command: {}", e),
                    }
                }
                "Test the project" => {
                    println!("T");

                    match std::process::Command::new("cargo").arg("test").output() {
                        Ok(output) => {
                            // Handle the output here
                            println!("Output: {}", String::from_utf8_lossy(&output.stdout));
                        }
                        Err(e) => println!("Failed to execute command: {}", e),
                    }
                }
                "Development environment setup" => {
                    println!("D");
                    setup();
                }
                _ => {}
            }
        }
        Err(_) => {
            println!("No choice was made")
        }
    }
}

fn print_logo() {
    println!(
        "

  @@@                        @@@                        @@@
  @@@@@                     @@@@@                     @@@@@@
 @@@@@@@@                  @@@*@@@                  @@@@@@@@
  @@@@@@@@@@             @@@%-:-@@@              @@@@@@@@@@
   @@@@@%%@@@@          @@@%-::::%@@@          @@@@%%@@@@@
    @@@@@%=#@@@@       @@@%-:::::-%@@@       @@@@#=#@@@@@
     @@@@@%--#@@@@    @@@#::::::::-%@@@    @@@@*--%@@@@@
      @@@@@%---*@@@@ @@@*:::::::::::#@@@ @@@@+--=@@@@@@
       @@@@@@=---+@@@@@+:::::=#=:::::*@@@@@+---=@@@@@@
        @@@@@@=----#@@+:::::+@@@=:::::+@@*----=@@@@@
          @@@@@=--#@@=:::::=@@@@@=:::::+@@*--+@@@@@
           @@@@@+#@@=:::::-@@#:#@@=:::::=@@#*@@@@@
            @@@@@@@-:::::-@@#---%@%-:::::-@@@@@@@
             @@@@%-::::::%@%=----%@@-:::::-@@@@@
             @@@#---::--%@%=======@@%-::::--%@@@
            @@@#------=@@@+=======*@@@=------#@@@
           @@@#-----+@@@%@@@#+++#@@@%@@@+-----#@@@
          @@@*----*@@@@-::+@@@@@@@+::=@@@@*----*@@@
         @@@+---#@@@@@@@=:::=@@@=:::=@@@@@@@*---*@@@
        @@@+--#@@@@@@@@@@=:::::::::+@@@@@@@@@@#--+@@@
       @@@==%@@@@@@@@@@@@@+:::::::+@@@@@@@@@@@@@%=+@@@
     @@@%+@@@@@@@@    @@@@@*:::::#@@@@@    @@@@@@@%*@@@
    @@@@@@@@@@@@       @@@@@*:::#@@@@@       @@@@@@@@@@@@
   @@@@@@@@@@@          @@@@@#-#@@@@@          @@@@@@@@@@@
  @@@@@@@@@@             @@@@@@@@@@@             @@@@@@@@@@
  @@@@@@@@                @@@@@@@@@                @@@@@@@@@
 @@@@@@                    @@@@@@@                   @@@@@@@
  @@@                        @@@                        @@@
    "
    );
}
