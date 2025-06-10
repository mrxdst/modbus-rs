use std::{error::Error, io::Write, sync::Arc, time::Duration};

use clap::Parser;
use comfy_table::{presets, CellAlignment, ColumnConstraint, Table, Width};
use modbus::{ModbusError, ModbusTCPClient};
use rustyline::{completion::Completer, history::MemHistory, Editor, Helper, Highlighter, Hinter, Validator};
use tokio::{net::TcpStream, select, sync::Mutex, time::Instant};

use crate::{
    args::*,
    util::{timeout_or_cancel, PrettyDisplay},
};

use super::{
    address::{Address, AddressKind},
    args::{Cli, ExportArgs, ReadArgs, ScanArgs},
};

pub async fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let host_port = format!("{}:{}", args.host, args.port);

    let mut client = ClientImpl::new(args.timeout, host_port);

    client.command_loop().await?;

    Ok(())
}

struct ClientImpl {
    timeout: Duration,
    host_port: String,
    client: Arc<Mutex<Option<Arc<ModbusTCPClient>>>>,
    last_table: Option<Table>,
    unit_id: u8,
    offset: i32,
}

impl ClientImpl {
    pub fn new(timeout: Duration, addr: String) -> Self {
        Self {
            timeout,
            host_port: addr,
            client: Arc::new(Mutex::new(None)),
            last_table: None,
            unit_id: 0,
            offset: -1,
        }
    }

    async fn command_loop(&mut self) -> Result<(), Box<dyn Error>> {
        self.connect_if_needed().await?;

        println!("unit-id = {}", self.unit_id);
        println!("offset = {}", self.offset);
        println!();

        let config = rustyline::Config::builder().build();
        let helper = InteractiveHelper {};

        let mut rl = Editor::<InteractiveHelper, MemHistory>::with_history(config, MemHistory::new())?;
        rl.set_helper(Some(helper));
        let rl = Arc::new(Mutex::new(rl));

        loop {
            let _rl = rl.clone();
            let readline = tokio::spawn(async move { _rl.lock().await.readline("modbus-test> ") }).await?;

            match readline {
                Ok(line) => {
                    _ = rl.lock().await.add_history_entry(line.as_str());

                    println!();

                    let result = self.handle_command(line).await;

                    if let Ok(true) = result {
                        return Ok(());
                    }

                    if let Err(err) = result {
                        println!("{err}");
                    }

                    println!();
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, line: String) -> Result<bool, Box<dyn Error>> {
        let words = shellwords::split(&format!("modbus-test> {}", line))?;

        let cmd = Interactive::try_parse_from(words)?;

        let start = Instant::now();

        let result = match &cmd.command {
            InteractiveCommands::Info => self.info().await,
            InteractiveCommands::Read(args) => self.read(args).await,
            InteractiveCommands::Write(args) => self.write(args).await,
            InteractiveCommands::Export(args) => self.export_csv(args).await,
            InteractiveCommands::Scan(args) => self.scan(args).await,
            InteractiveCommands::Set(args) => match args.command {
                SetCommands::UnitId { unit_id } => {
                    self.unit_id = unit_id;
                    println!("unit-id = {unit_id}");
                    return Ok(false);
                }
                SetCommands::Offset { offset } => {
                    self.offset = offset;
                    println!("offset = {offset}");
                    return Ok(false);
                }
                SetCommands::Timeout { timeout } => {
                    self.timeout = timeout;
                    println!("timeout = {}ms", timeout.as_millis());
                    return Ok(false);
                }
            },
            InteractiveCommands::Exit => return Ok(true),
        };

        let dur = Instant::now() - start;

        println!();
        println!("{}: {}ms", cmd.command, dur.as_millis());

        result.map(|_| false)
    }

    async fn info(&mut self) -> Result<(), Box<dyn Error>> {
        let client = self.connect_if_needed().await?;

        let result = timeout_or_cancel(self.timeout, client.read_device_identification(self.unit_id)).await??;

        let mut table = Table::new();
        table.load_preset(presets::NOTHING);
        table.set_header(["Description", "Value"]);

        table.add_row(["VendorName", result.vendor_name.as_ref()]);
        table.add_row(["ProductCode", result.product_code.as_ref()]);
        table.add_row(["MajorMinorRevision", result.major_minor_revision.as_ref()]);
        if let Some(vendor_url) = result.vendor_url {
            table.add_row(["VendorUrl", vendor_url.as_ref()]);
        }
        if let Some(product_name) = result.product_name {
            table.add_row(["ProductName", product_name.as_ref()]);
        }
        if let Some(model_name) = result.model_name {
            table.add_row(["ModelName", model_name.as_ref()]);
        }
        if let Some(user_application_name) = result.user_application_name {
            table.add_row(["UserApplicationName", user_application_name.as_ref()]);
        }

        for (id, data) in result.objects {
            let data = data.iter().map(|v| format!("{v:#x}")).collect::<Vec<String>>().join(", ");
            table.add_row([format!("{id:#x}"), format!("{data}")]);
        }

        println!("{table}");

        self.last_table = Some(table);

        Ok(())
    }

    async fn scan(&self, args: &ScanArgs) -> Result<(), Box<dyn Error>> {
        let client = self.connect_if_needed().await?;

        let do_scan = || async {
            let mut table = Table::new();
            table.load_preset(presets::NOTHING);
            table.set_header(["Unit", "Coils", "Discrete inputs", "Input registers", "Holding registers"]);
            let unit_col = ColumnConstraint::Absolute(Width::Fixed(8));
            let result_col = ColumnConstraint::Absolute(Width::Fixed(25));
            table.column_mut(0).unwrap().set_constraint(unit_col);
            table.column_iter_mut().skip(1).for_each(|c| {
                c.set_constraint(result_col);
            });

            println!("{table}");

            for unit_id in args.min..=args.max {
                let mut table = Table::new();
                table.load_preset(presets::NOTHING);

                let coils: String = match timeout_or_cancel(self.timeout, client.read_coils(unit_id, 0, 1)).await {
                    Err(reason) => reason.to_string(),
                    Ok(Ok(_)) => "Good".into(),
                    Ok(Err(ModbusError::ModbusException(ex))) => format!("{ex:?}"),
                    Ok(Err(err)) => return Err(err),
                };

                let discrete_inputs: String = match timeout_or_cancel(self.timeout, client.read_discrete_inputs(unit_id, 0, 1)).await {
                    Err(reason) => reason.to_string(),
                    Ok(Ok(_)) => "Good".into(),
                    Ok(Err(ModbusError::ModbusException(ex))) => format!("{ex:?}"),
                    Ok(Err(err)) => return Err(err),
                };

                let input_registers: String = match timeout_or_cancel(self.timeout, client.read_input_registers(unit_id, 0, 1)).await {
                    Err(reason) => reason.to_string(),
                    Ok(Ok(_)) => "Good".into(),
                    Ok(Err(ModbusError::ModbusException(ex))) => format!("{ex:?}"),
                    Ok(Err(err)) => return Err(err),
                };

                let holding_registers: String = match timeout_or_cancel(self.timeout, client.read_holding_registers(unit_id, 0, 1)).await {
                    Err(reason) => reason.to_string(),
                    Ok(Ok(_)) => "Good".into(),
                    Ok(Err(ModbusError::ModbusException(ex))) => format!("{ex:?}"),
                    Ok(Err(err)) => return Err(err),
                };

                table.add_row([unit_id.to_string(), coils, discrete_inputs, input_registers, holding_registers]);
                table.column_mut(0).unwrap().set_constraint(unit_col);
                table.column_iter_mut().skip(1).for_each(|c| {
                    c.set_constraint(result_col);
                });

                println!("{table}");
            }

            Ok(())
        };

        select! {
            _ = do_scan() => {}
            _ = tokio::signal::ctrl_c() => {}
        };

        Ok(())
    }

    async fn read(&mut self, args: &ReadArgs) -> Result<(), Box<dyn Error>> {
        let address = Address::parse(&args.address, self.offset)?;

        let client = self.connect_if_needed().await?;

        enum ResultType {
            Coils(Vec<bool>),
            Registers(Vec<u16>),
        }

        let result = match address.kind {
            AddressKind::Coil => {
                ResultType::Coils(timeout_or_cancel(self.timeout, client.read_coils(self.unit_id, address.index, args.length)).await??)
            }
            AddressKind::DiscreteInput => {
                ResultType::Coils(timeout_or_cancel(self.timeout, client.read_discrete_inputs(self.unit_id, address.index, args.length)).await??)
            }
            AddressKind::InputRegister => {
                ResultType::Registers(timeout_or_cancel(self.timeout, client.read_input_registers(self.unit_id, address.index, args.length)).await??)
            }
            AddressKind::HoldingRegister => ResultType::Registers(
                timeout_or_cancel(self.timeout, client.read_holding_registers(self.unit_id, address.index, args.length)).await??,
            ),
        };

        match result {
            ResultType::Coils(values) => self.print_coils(address, &values),
            ResultType::Registers(values) => self.print_registers(address, &values, args.show64bit),
        }

        Ok(())
    }

    async fn write(&self, args: &WriteArgs) -> Result<(), Box<dyn Error>> {
        let address = Address::parse(&args.address, self.offset)?;

        match address.kind {
            AddressKind::DiscreteInput | AddressKind::InputRegister => return Err("Address must start with 0 or 4.".into()),
            AddressKind::Coil => {
                let mut values: Vec<bool> = vec![];

                for value in args.values.iter() {
                    let value: bool = value.to_lowercase().parse()?;
                    values.push(value);
                }

                let client = self.connect_if_needed().await?;

                timeout_or_cancel(self.timeout, client.write_multiple_coils(self.unit_id, address.index, &values)).await??;

                println!("Wrote {} value(s)", args.values.len());
            }
            AddressKind::HoldingRegister => {
                let mut values: Vec<u16> = vec![];

                for value in args.values.iter() {
                    let bytes = match args.datatype {
                        WriteDatatype::U16 => value.parse::<u16>()?.to_be_bytes().to_vec(),
                        WriteDatatype::I16 => value.parse::<i16>()?.to_be_bytes().to_vec(),
                        WriteDatatype::U32 => value.parse::<u32>()?.to_be_bytes().to_vec(),
                        WriteDatatype::I32 => value.parse::<i32>()?.to_be_bytes().to_vec(),
                        WriteDatatype::F32 => value.parse::<f32>()?.to_be_bytes().to_vec(),
                        WriteDatatype::U64 => value.parse::<u64>()?.to_be_bytes().to_vec(),
                        WriteDatatype::I64 => value.parse::<i64>()?.to_be_bytes().to_vec(),
                        WriteDatatype::F64 => value.parse::<f64>()?.to_be_bytes().to_vec(),
                        WriteDatatype::Hex => u16::from_str_radix(&value, 16)?.to_be_bytes().to_vec(),
                        WriteDatatype::Bin => u16::from_str_radix(&value, 2)?.to_be_bytes().to_vec(),
                    };
                    match args.order {
                        WriteOrder::HL => {
                            for i in 0..bytes.len() / 2 {
                                values.push(u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]));
                            }
                        }
                        WriteOrder::LH => {
                            for i in (0..bytes.len() / 2).rev() {
                                values.push(u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]));
                            }
                        }
                    }
                }

                let client = self.connect_if_needed().await?;

                timeout_or_cancel(
                    self.timeout,
                    client.write_multiple_holding_registers(self.unit_id, address.index, &values),
                )
                .await??;

                println!("Wrote {} value(s)", args.values.len());
            }
        }

        Ok(())
    }

    fn print_coils(&mut self, address: Address, values: &Vec<bool>) {
        let mut table = Table::new();
        table.load_preset(presets::NOTHING);
        table.set_header(["Address", "Value"]);

        for (offset, value) in values.iter().enumerate() {
            let index: i32 = address.index as i32 + offset as i32 - self.offset;
            let prefix = if address.kind == AddressKind::Coil { "0" } else { "1" };
            table.add_row([format!("{prefix}{index:05}"), value.to_string().to_uppercase()]);
        }

        println!("{table}");

        self.last_table = Some(table);
    }

    fn print_registers(&mut self, address: Address, values: &Vec<u16>, show64bit: bool) {
        let show32bit = if show64bit { true } else { values.len() > 1 };

        let mut table = Table::new();
        table.load_preset(presets::NOTHING);

        let mut header = Vec::with_capacity(17);
        header.push("Address");
        header.push("U16");
        header.push("I16");
        if show32bit {
            header.push("U32[HL]");
            header.push("U32[LH]");
            header.push("I32[HL]");
            header.push("I32[LH]");
            header.push("F32[HL]");
            header.push("F32[LH]");
        }
        if show64bit {
            header.push("U64[HL]");
            header.push("U64[LH]");
            header.push("I64[HL]");
            header.push("I64[LH]");
            header.push("F64[HL]");
            header.push("F64[LH]");
        }
        header.push("Hex");
        header.push("Bin");

        let column_count = header.len();
        table.set_header(header);

        table.column_iter_mut().skip(1).for_each(|c| c.set_cell_alignment(CellAlignment::Right));
        table
            .column_iter_mut()
            .skip(column_count - 2)
            .for_each(|c| c.set_cell_alignment(CellAlignment::Left));

        for (offset, value) in values.iter().enumerate() {
            let index: i32 = address.index as i32 + offset as i32 - self.offset;
            let prefix = if address.kind == AddressKind::InputRegister { "3" } else { "4" };

            let value2 = values.get(offset + 1);
            let value3 = values.get(offset + 2);
            let value4 = values.get(offset + 3);

            let mut row: Vec<String> = Vec::with_capacity(column_count);
            let empty = "-------";

            row.push(format!("{prefix}{index:05}")); // Address
            row.push(format!("{}", *value)); // U16
            row.push(format!("{}", i16::from_be_bytes(value.to_be_bytes()))); // I16

            if show32bit {
                if let Some(value2) = value2 {
                    let b0 = value.to_be_bytes()[0];
                    let b1 = value.to_be_bytes()[1];
                    let b2 = value2.to_be_bytes()[0];
                    let b3 = value2.to_be_bytes()[1];

                    row.push(format!("{}", u32::from_be_bytes([b0, b1, b2, b3]))); // U32HL
                    row.push(format!("{}", u32::from_be_bytes([b2, b3, b0, b1]))); // U32LH
                    row.push(format!("{}", i32::from_be_bytes([b0, b1, b2, b3]))); // I32HL
                    row.push(format!("{}", i32::from_be_bytes([b2, b3, b0, b1]))); // I32LH
                    row.push(f32::from_be_bytes([b0, b1, b2, b3]).pretty()); // F32HL
                    row.push(f32::from_be_bytes([b2, b3, b0, b1]).pretty()); // F32LH

                    if show64bit {
                        if let (Some(value3), Some(value4)) = (value3, value4) {
                            let b4 = value3.to_be_bytes()[0];
                            let b5 = value3.to_be_bytes()[1];
                            let b6 = value4.to_be_bytes()[0];
                            let b7 = value4.to_be_bytes()[1];

                            row.push(format!("{}", u64::from_be_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))); // U64HL
                            row.push(format!("{}", u64::from_be_bytes([b6, b7, b4, b5, b2, b3, b0, b1]))); // U64LH
                            row.push(format!("{}", i64::from_be_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))); // I64HL
                            row.push(format!("{}", i64::from_be_bytes([b6, b7, b4, b5, b2, b3, b0, b1]))); // I64LH
                            row.push(f64::from_be_bytes([b0, b1, b2, b3, b4, b5, b6, b7]).pretty()); // F64HL
                            row.push(f64::from_be_bytes([b6, b7, b4, b5, b2, b3, b0, b1]).pretty());
                        // F64LH
                        } else {
                            for _ in 0..6 {
                                row.push(empty.into());
                            }
                        }
                    }
                } else {
                    for _ in 0..6 {
                        row.push(empty.into());
                    }
                    if show64bit {
                        for _ in 0..6 {
                            row.push(empty.into());
                        }
                    }
                }
            }

            row.push(format!("{:04X}", *value)); // Hex
            row.push(format!(
                "{:04b} {:04b} {:04b} {:04b}",
                *value >> 12 & 0xF,
                *value >> 8 & 0xF,
                *value >> 4 & 0xF,
                *value & 0xF
            )); // Bin

            table.add_row(row);
        }

        println!("{table}");

        self.last_table = Some(table);
    }

    async fn export_csv(&self, args: &ExportArgs) -> Result<(), Box<dyn Error>> {
        let table = match &self.last_table {
            Some(table) => table,
            None => {
                println!("Nothing to export");
                return Ok(());
            }
        };

        let mut writer = csv::Writer::from_path(&args.filename)?;

        let header = table.header().unwrap().cell_iter().map(|c| c.content()).collect::<Vec<String>>();
        writer.write_record(header)?;

        for row in table.row_iter() {
            let record = row.cell_iter().map(|c| c.content()).collect::<Vec<String>>();
            writer.write_record(record)?;
        }
        writer.flush()?;

        println!("Exported");

        Ok(())
    }

    async fn connect_if_needed(&self) -> Result<Arc<ModbusTCPClient>, Box<dyn Error>> {
        if let Some(client) = self.client.lock().await.as_ref() {
            return Ok(client.clone());
        }

        print!("Connecting...");
        std::io::stdout().flush()?;

        let stream = timeout_or_cancel(self.timeout, TcpStream::connect(&self.host_port)).await??;

        println!(" Connected");
        println!();

        let (client, handle) = ModbusTCPClient::new(stream);

        let client = Arc::new(client);

        _ = self.client.lock().await.insert(client.clone());

        let client_ = self.client.clone();

        tokio::spawn(async move {
            let result = handle.await.unwrap_or(Ok(()));
            _ = client_.lock().await.take();
            println!();
            println!();
            match result {
                Ok(_) => println!("Connection closed"),
                Err(err) => println!("{err}"),
            }
            println!();
        });

        Ok(client)
    }
}

#[derive(Helper, Hinter, Validator, Highlighter)]
struct InteractiveHelper {}
const COMPLETIONS: [&str; 10] = [
    "info",
    "scan ",
    "read ",
    "write ",
    "set offset ",
    "set timeout ",
    "set unit-id ",
    "export ",
    "help",
    "exit",
];

impl Completer for InteractiveHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut matches = vec![];

        for cmd in COMPLETIONS {
            if cmd.starts_with(line) {
                matches.push(String::from(&cmd[pos..]));
            }
        }

        let _ = (line, pos, ctx);
        Ok((pos, matches))
    }
}
