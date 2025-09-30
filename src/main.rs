use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::{config::TransmitConfig, TxRmtDriver};

mod led;
use led::{Ws2812Led, RgbColor};


fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32-S3 RGB LED 控制程序启动!");

    // 获取外设
    let peripherals = Peripherals::take().unwrap();
    
    // ESP32-S3 RGB LED 引脚配置 - 使用GPIO48
    // 根据ESP32-S3硬件，RGB LED连接在GPIO48
    let led_pin = peripherals.pins.gpio48;
    
    // 配置RMT传输
    let config = TransmitConfig::new()
        .clock_divider(1);  // 时钟分频器 - 高分辨率
    
    // 创建RMT传输驱动
    let rmt = TxRmtDriver::new(
        peripherals.rmt.channel0,
        led_pin,
        &config,
    ).unwrap();
    
    // 创建LED控制器
    let mut led = Ws2812Led::new(rmt);
    
    // 确保所有LED初始状态为关闭
    log::info!("初始化LED状态 - 确保所有LED关闭");
    led.set_color(RgbColor::black()).unwrap();
    FreeRtos::delay_ms(1000);
    
    // 测试WS2812 LED颜色控制
    log::info!("测试WS2812 LED颜色控制...");
    log::info!("使用RMT驱动，GPIO48引脚");
    
    // 首先确保所有LED完全关闭
    log::info!("确保所有LED完全关闭");
    led.set_color(RgbColor::black()).unwrap();
    FreeRtos::delay_ms(2000);
    
    // 测试基本颜色
    log::info!("测试红色");
    led.set_color(RgbColor::red()).unwrap();
    FreeRtos::delay_ms(2000);
    
    log::info!("测试绿色");
    led.set_color(RgbColor::green()).unwrap();
    FreeRtos::delay_ms(2000);
    
    log::info!("测试蓝色");
    led.set_color(RgbColor::blue()).unwrap();
    FreeRtos::delay_ms(2000);
    
    // 测试渐变效果
    log::info!("测试渐变效果 - 从红色到蓝色");
    led.fade_to(RgbColor::blue(), 3000, 30).unwrap();
    FreeRtos::delay_ms(1000);
    
    log::info!("测试渐变效果 - 从蓝色到绿色");
    led.fade_to(RgbColor::green(), 3000, 30).unwrap();
    FreeRtos::delay_ms(1000);
    
    // 测试呼吸灯效果
    log::info!("测试呼吸灯效果");
    led.breathing(RgbColor::red(), 2).unwrap();
    FreeRtos::delay_ms(1000);
    
    // 测试闪烁效果
    log::info!("测试闪烁效果");
    led.blink(RgbColor::blue(), 5, 200, 200).unwrap();
    FreeRtos::delay_ms(1000);
    
    // 最终关闭所有LED
    log::info!("最终关闭所有LED");
    led.set_color(RgbColor::black()).unwrap();
    FreeRtos::delay_ms(2000);
    
    log::info!("RGB LED 初始化完成，开始主循环");
    FreeRtos::delay_ms(2000);
    
    // 主循环 - 演示LED控制
    let mut color_index = 0;
    let colors = [
        RgbColor::red(),
        RgbColor::green(),
        RgbColor::blue(),
        RgbColor::new(255, 255, 0),  // 黄色
        RgbColor::new(255, 0, 255),  // 洋红色
        RgbColor::new(0, 255, 255),  // 青色
        RgbColor::black(),           // 关闭
    ];
    
    loop {
        let target_color = colors[color_index];
        
        log::info!("渐变到颜色: {:?}", target_color);
        led.fade_to(target_color, 2000, 20).unwrap();
        
        color_index = (color_index + 1) % colors.len();
    }
    
}
