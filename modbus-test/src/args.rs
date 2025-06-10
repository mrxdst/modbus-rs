use std::{fmt::Display, num::ParseIntError, path::PathBuf, time::Duration};

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Hostname or ip address. (or "server" to run a test server).
    pub host: String,

    /// TCP port number
    #[arg(default_value = "502")]
    pub port: u16,

    /// Network timeout in ms
    #[arg(short, long, default_value = "2000", value_parser = parse_duration)]
    pub timeout: Duration,
}

#[derive(Parser, Debug)]
#[command()]
pub struct Interactive {
    #[command(subcommand)]
    pub command: InteractiveCommands,
}

#[derive(Subcommand, Debug)]
pub enum InteractiveCommands {
    /// Read info from device
    Info,

    /// Read values from device
    Read(ReadArgs),

    /// Write values to device
    Write(WriteArgs),

    /// Export the previously printed table
    Export(ExportArgs),

    /// Scan for devices
    Scan(ScanArgs),

    /// Set configuration
    Set(SetArgs),

    /// Exit the program
    Exit,
}

impl Display for InteractiveCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractiveCommands::Info => write!(f, "Info"),
            InteractiveCommands::Read(_) => write!(f, "Read"),
            InteractiveCommands::Write(_) => write!(f, "Write"),
            InteractiveCommands::Export(_) => write!(f, "Export"),
            InteractiveCommands::Scan(_) => write!(f, "Scan"),
            InteractiveCommands::Set(_) => write!(f, "Set"),
            InteractiveCommands::Exit => write!(f, "Exit"),
        }
    }
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    /// Address to start reading from
    pub address: String,

    /// Number of addresses to read
    #[arg(default_value = "1")]
    pub length: u16,

    /// Show 64-bit data-types
    #[arg(long = "64", id = "64-bit")]
    pub show64bit: bool,
}

#[derive(Args, Debug)]
#[command(allow_negative_numbers = true)]
pub struct WriteArgs {
    /// Address to start writing to
    pub address: String,

    /// Values to write
    #[arg(required = true)]
    pub values: Vec<String>,

    /// Datatype of the values
    #[arg(long = "type", value_enum, default_value = "I16")]
    pub datatype: WriteDatatype,

    /// Multi-register order
    #[arg(long = "order", value_enum, default_value = "HL")]
    pub order: WriteOrder,
}

#[derive(Debug, PartialEq, Clone, Copy, ValueEnum)]
#[value(rename_all = "PascalCase")]
pub enum WriteDatatype {
    U16,
    I16,
    U32,
    I32,
    F32,
    U64,
    I64,
    F64,
    Hex,
    Bin,
}

#[derive(Debug, PartialEq, Clone, Copy, ValueEnum)]
#[value(rename_all = "UPPER")]
pub enum WriteOrder {
    /// First word low
    LH,

    /// First word high
    HL,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// The file to write to
    pub filename: PathBuf,
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Minimum unit id
    #[arg(default_value = "0")]
    pub min: u8,

    /// Maximum unit id
    #[arg(default_value = "255")]
    pub max: u8,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    #[command(subcommand)]
    pub command: SetCommands,
}

#[derive(Subcommand, Debug)]
pub enum SetCommands {
    /// Set the unit-id
    UnitId { unit_id: u8 },

    /// Set timeout
    Timeout {
        #[arg(value_parser = parse_duration)]
        timeout: Duration,
    },

    /// Set address offset
    #[command(allow_negative_numbers = true)]
    Offset { offset: i32 },
}

fn parse_duration(input: &str) -> Result<Duration, ParseIntError> {
    let ms = input.parse()?;
    Ok(Duration::from_millis(ms))
}
