use esp_idf_hal::rmt::{Pulse, RxRmtDriver};
use esp_idf_hal::rmt::config::ReceiveConfig;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::rmt::{config::TransmitConfig, TxRmtDriver};

mod led;
mod bluetooth;
use led::{Ws2812Led, RgbColor};
use bluetooth::BluetoothManager;


fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP32-S3 RGB LED 控制程序启动!");

    // 获取外设
    let peripherals = Peripherals::take().unwrap();
    
    // 创建系统事件循环
    let _sys_loop = esp_idf_svc::eventloop::EspSystemEventLoop::take().unwrap();
    
    // 创建NVS分区
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take().unwrap();

    // 初始化蓝牙驱动
    let bt = std::sync::Arc::new(esp_idf_svc::bt::BtDriver::new(peripherals.modem, Some(nvs.clone())).unwrap());
    
    // 创建GAP和GATTS
    let gap = std::sync::Arc::new(esp_idf_svc::bt::ble::gap::EspBleGap::new(bt.clone()).unwrap());
    let gatts = std::sync::Arc::new(esp_idf_svc::bt::ble::gatt::server::EspGatts::new(bt.clone()).unwrap());

    // 初始化蓝牙管理器
    let bluetooth_manager = BluetoothManager::new(gap, gatts);
    match bluetooth_manager.initialize() {
        Ok(_) => {
            log::info!("BLE GATT服务器初始化成功!");
            bluetooth_manager.start_data_receiver();
        }
        Err(e) => {
            log::error!("BLE初始化失败: {:?}", e);
        }
    }
    
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
    

    // 红外接收配置
    let ir_recv_pin = peripherals.pins.gpio21;


    
    let receive_config = ReceiveConfig::new()
        .idle_threshold(10000u16);  // 空闲阈值 - 10ms空闲后认为信号结束
        // .carrier(Some(CarrierConfig::new().carrier_level(PinState::High)));
    
    const STACK_SIZE: usize = 250;
    // 创建RMT接收驱动
    let mut ir_receiver = RxRmtDriver::new(
        peripherals.rmt.channel4,
        ir_recv_pin,
        &receive_config,
        STACK_SIZE,  // 缓冲区大小
    ).unwrap();
    
    log::info!("红外接收器初始化完成，开始监听...");
    log::info!("IR接收器引脚: GPIO44");
    log::info!("RMT通道: Channel4");
    log::info!("时钟分频: 80, 空闲阈值: 10000, 滤波器: 启用");
    
    // 启动RMT接收
    ir_receiver.start().unwrap();
    log::info!("RMT接收已启动");
    
    // 主循环 - 持续监听红外信号和蓝牙数据
    let mut connection_check_counter = 0;
    loop {
        // 检查蓝牙连接状态
        if bluetooth_manager.is_connected() {
            if connection_check_counter % 100 == 0 {  // 每10秒打印一次
                log::info!("蓝牙已连接");
            }
            
            // 处理接收到的蓝牙数据
            let bluetooth_data = bluetooth_manager.get_received_data();
            if !bluetooth_data.is_empty() {
                log::info!("接收到蓝牙数据: {:?}", bluetooth_data);
                
                // 将蓝牙数据转换为字符串并记录
                if let Ok(data_str) = String::from_utf8(bluetooth_data.clone()) {
                    log::info!("蓝牙数据内容: {}", data_str);
                    
                    // 根据接收到的数据控制LED
                    match data_str.trim() {
                        "red" => {
                            log::info!("设置LED为红色");
                            led.set_color(RgbColor::red()).unwrap();
                        }
                        "green" => {
                            log::info!("设置LED为绿色");
                            led.set_color(RgbColor::green()).unwrap();
                        }
                        "blue" => {
                            log::info!("设置LED为蓝色");
                            led.set_color(RgbColor::blue()).unwrap();
                        }
                        "off" => {
                            log::info!("关闭LED");
                            led.set_color(RgbColor::black()).unwrap();
                        }
                        _ => {
                            log::info!("未知的LED命令: {}", data_str);
                        }
                    }
                }
            }
        } else {
            if connection_check_counter % 100 == 0 {  // 每10秒打印一次
                log::info!("蓝牙未连接，等待连接...");
            }
        }
        
        connection_check_counter += 1;
        
        // // 准备接收缓冲区 - 使用正确的初始化
        // let mut pulses = [(
        //     Pulse::zero(), Pulse::zero()
        // ); STACK_SIZE];
        
        // // 等待红外信号 (1000 ticks = 约1秒，假设tick rate为1000Hz)
        // match ir_receiver.receive(&mut pulses, 1000) {
        //     Ok(received_count) => {
        //         match received_count {
        //             esp_idf_hal::rmt::Receive::Read(count) => {
        //                 log::info!("接收到红外信号，脉冲数量: {}", count);
        //                 let pulses = &pulses[..count];

        //                 for (pulse0, pulse1) in pulses {
        //                     log::info!("0={pulse0:?}, 1={pulse1:?}");
        //                 }
                        
        //                 // 如果蓝牙已连接，发送红外数据到蓝牙
        //                 if bluetooth_manager.is_connected() {
        //                     let ir_data = format!("IR: {} pulses", count);
        //                     if let Err(e) = bluetooth_manager.send_data(ir_data.as_bytes()) {
        //                         log::error!("发送红外数据到蓝牙失败: {:?}", e);
        //                     }
        //                 }
        //             }
        //             esp_idf_hal::rmt::Receive::Overflow(count) => {
        //                 log::warn!("接收缓冲区溢出，脉冲数量: {}", count);
        //             }
        //             esp_idf_hal::rmt::Receive::Timeout => {
        //                 // 不记录超时，减少日志输出
        //             }
        //         }
        //     }
        //     Err(e) => {
        //         log::error!("RMT接收错误: {:?}", e);
        //     }
        // }
        
        // 短暂延时
        FreeRtos::delay_ms(100);
    }
}

