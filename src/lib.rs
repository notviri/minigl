#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "windows")]
pub use win32::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        super::do_it();
    }
}
