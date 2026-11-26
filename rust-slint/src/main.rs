fn main() {
    #[cfg(not(target_os = "android"))]
    notepad::run(None);
}
