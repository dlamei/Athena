use atlas::Atlas;

fn main() {

    #[cfg(not(target_arch="wasm32"))]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            unsafe {
                std::env::remove_var("WAYLAND_DISPLAY");
            }
        }

        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Warn)
    //     .filter_module("atlas", log::LevelFilter::Info)
    //     // .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
    //     .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Warn)
    //     .format_timestamp(None)
    //     .init();

    Atlas::init().run().unwrap();

    // let mut jit = compiler::jit::JITCompiler::init();
    // let f1 = jit.compile_for_f64("fn_f64", &code);
    // let f2 = jit.compile_for_f64x2xn("fn_f64x2xn", &code);

    // let n = 1028 << 16;

    // let start = std::time::Instant::now();
    // for i in 0..n {
    //     let _ = f1(i as f64, i as f64);
    // }
    // let end = std::time::Instant::now();
    // println!("time: {}", (end-start).as_millis());

    // // let start = std::time::Instant::now();
    // // for i in 0..n {
    // //     let j = (i*8) as f64;
    // //     let a = [j, j+1., j+2., j+3., j+4., j+5., j+6., j+7.];
    // //     let mut o = [0.0;8];
    // //     let _ = f2(&a, &a, &mut o);
    // // }
    // // let end = std::time::Instant::now();
    // // println!("time: {}", (end-start).as_millis());
    // let inp: Vec<_> = (0..n).into_iter().map(|i| i as f64).collect();
    // let mut out = vec![0.0;n];
    // let start = std::time::Instant::now();
    // f2(inp.as_ptr(), inp.as_ptr(), out.as_mut_ptr(), n as i64);
    // let end = std::time::Instant::now();
    // println!("time: {}", (end-start).as_millis());
}
