use esp_idf_svc::hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver};
use esp_idf_svc::hal::delay::FreeRtos;
use std::time::Duration;

/// RGB颜色结构体
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    /// 创建新的RGB颜色
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
    
    /// 创建黑色（关闭）
    pub fn black() -> Self {
        Self { red: 0, green: 0, blue: 0 }
    }
    
    /// 创建白色
    pub fn white() -> Self {
        Self { red: 255, green: 255, blue: 255 }
    }
    
    /// 创建红色
    pub fn red() -> Self {
        Self { red: 255, green: 0, blue: 0 }
    }
    
    /// 创建绿色
    pub fn green() -> Self {
        Self { red: 0, green: 255, blue: 0 }
    }
    
    /// 创建蓝色
    pub fn blue() -> Self {
        Self { red: 0, green: 0, blue: 255 }
    }
    
    /// 线性插值计算两个颜色之间的中间颜色
    pub fn lerp(&self, other: &RgbColor, t: f32) -> RgbColor {
        let t = t.clamp(0.0, 1.0);
        RgbColor {
            red: ((1.0 - t) * self.red as f32 + t * other.red as f32) as u8,
            green: ((1.0 - t) * self.green as f32 + t * other.green as f32) as u8,
            blue: ((1.0 - t) * self.blue as f32 + t * other.blue as f32) as u8,
        }
    }
}

/// WS2812 LED控制器
pub struct Ws2812Led {
    rmt: TxRmtDriver<'static>,
    current_color: RgbColor,
}

impl Ws2812Led {
    /// 创建新的WS2812 LED控制器
    pub fn new(rmt: TxRmtDriver<'static>) -> Self {
        Self {
            rmt,
            current_color: RgbColor::black(),
        }
    }
    
    /// 设置LED颜色
    pub fn set_color(&mut self, color: RgbColor) -> Result<(), Box<dyn std::error::Error>> {
        // log::info!("设置LED颜色: R={}, G={}, B={}", color.red, color.green, color.blue);
        
        // 获取RMT时钟频率
        let ticks_hz = self.rmt.counter_clock()?;
        
        // 创建正确的WS2812时序脉冲
        let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &Duration::from_nanos(350))?;
        let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &Duration::from_nanos(800))?;
        let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &Duration::from_nanos(700))?;
        let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &Duration::from_nanos(600))?;
        
        // 构建24位信号
        let mut signal = FixedLengthSignal::<24>::new();
        
        // 按照GRB顺序编码颜色数据
        let color_data: u32 = ((color.green as u32) << 16) | ((color.red as u32) << 8) | (color.blue as u32);
        
        // 从最高位开始设置每一位
        for i in (0..24).rev() {
            let bit_mask = 2_u32.pow(i);
            let bit: bool = (bit_mask & color_data) != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            
            signal.set(23 - i as usize, &(high_pulse, low_pulse))?;
        }
        
        // 发送信号
        self.rmt.start_blocking(&signal)?;
        self.current_color = color;
        
        log::info!("RMT信号发送成功");
        Ok(())
    }
    
    /// 获取当前颜色
    pub fn current_color(&self) -> RgbColor {
        self.current_color
    }
    
    /// 渐变到目标颜色
    pub fn fade_to(&mut self, target_color: RgbColor, duration_ms: u32, steps: u32) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("开始渐变: 从 {:?} 到 {:?}, 持续时间: {}ms, 步数: {}", 
                  self.current_color, target_color, duration_ms, steps);
        
        let step_duration = duration_ms / steps;
        
        for step in 0..=steps {
            let t = if steps == 0 { 1.0 } else { step as f32 / steps as f32 };
            let intermediate_color = self.current_color.lerp(&target_color, t);
            
            self.set_color(intermediate_color)?;
            FreeRtos::delay_ms(step_duration);
        }
        
        // 确保最终颜色准确
        self.set_color(target_color)?;
        log::info!("渐变完成");
        Ok(())
    }
    
    /// 彩虹渐变效果
    pub fn rainbow(&mut self, duration_ms: u32) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("开始彩虹渐变效果");
        
        let colors = [
            RgbColor::red(),
            RgbColor::new(255, 127, 0), // 橙色
            RgbColor::new(255, 255, 0), // 黄色
            RgbColor::green(),
            RgbColor::new(0, 255, 255), // 青色
            RgbColor::blue(),
            RgbColor::new(127, 0, 255), // 紫色
        ];
        
        let step_duration = duration_ms / colors.len() as u32;
        
        for color in colors.iter() {
            self.fade_to(*color, step_duration, 20)?;
        }
        
        log::info!("彩虹渐变完成");
        Ok(())
    }
    
    /// 呼吸灯效果
    pub fn breathing(&mut self, color: RgbColor, cycles: u32) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("开始呼吸灯效果: {:?}, 循环次数: {}", color, cycles);
        
        for _ in 0..cycles {
            // 从暗到亮
            for brightness in 0..=100 {
                let t = brightness as f32 / 100.0;
                let dimmed_color = RgbColor::black().lerp(&color, t);
                self.set_color(dimmed_color)?;
                FreeRtos::delay_ms(20);
            }
            
            // 从亮到暗
            for brightness in (0..=100).rev() {
                let t = brightness as f32 / 100.0;
                let dimmed_color = RgbColor::black().lerp(&color, t);
                self.set_color(dimmed_color)?;
                FreeRtos::delay_ms(20);
            }
        }
        
        // 最后关闭
        self.set_color(RgbColor::black())?;
        log::info!("呼吸灯效果完成");
        Ok(())
    }
    
    /// 闪烁效果
    pub fn blink(&mut self, color: RgbColor, times: u32, on_duration_ms: u32, off_duration_ms: u32) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("开始闪烁效果: {:?}, 次数: {}", color, times);
        
        for _ in 0..times {
            self.set_color(color)?;
            FreeRtos::delay_ms(on_duration_ms);
            self.set_color(RgbColor::black())?;
            FreeRtos::delay_ms(off_duration_ms);
        }
        
        log::info!("闪烁效果完成");
        Ok(())
    }
}
