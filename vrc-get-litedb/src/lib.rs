mod lowlevel;

extern "C" {
    pub fn add_dotnet(a: u32, b: u32) -> u32;
}

#[export_name = "rust_callback"]
extern "C" fn rust_callback() -> u32 {
    return 3;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_callback_test() {
        assert_eq!(rust_callback(), 3);
        // 1 + 2 + 3 = 6
        unsafe {
            assert_eq!(add_dotnet(1, 2), 6);
        }
    }
}
