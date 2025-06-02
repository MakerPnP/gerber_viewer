use std::io::BufWriter;

use gerber_types::{Command, GerberCode};

pub fn dump_gerber_source(commands: &Vec<Command>) {
    let gerber_source = gerber_commands_to_source(commands);

    println!("Gerber source:\n{}", gerber_source);
}

pub fn gerber_commands_to_source(commands: &Vec<Command>) -> String {
    let mut buf = BufWriter::new(Vec::new());
    commands
        .serialize(&mut buf)
        .expect("Could not generate Gerber code");
    let bytes = buf.into_inner().unwrap();
    let gerber_source = String::from_utf8(bytes).unwrap();
    gerber_source
}
