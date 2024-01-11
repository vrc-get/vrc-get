use std::env::args;

fn main() {
    let mut args = args();
    let a = args.next().unwrap().parse().unwrap();
    let b = args.next().unwrap().parse().unwrap();
    unsafe { vrc_get_litedb::add_dotnet(a, b) };
}
