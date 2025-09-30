# ESP32 Web Bluetooth 控制台

这是一个基于Web Bluetooth API的ESP32控制界面，允许你通过浏览器直接控制ESP32设备，无需安装任何应用程序。

## 🌟 功能特性

- **🔵 蓝牙连接**: 通过Web Bluetooth API直接连接ESP32
- **🎨 LED控制**: 一键控制RGB LED颜色（红、绿、蓝、关闭）
- **💬 自定义命令**: 发送任意文本命令到ESP32
- **📊 实时日志**: 查看设备连接状态和数据传输日志
- **📱 响应式设计**: 支持手机、平板、电脑等各种设备
- **🎯 实时通知**: 操作反馈和状态提示

## 🚀 快速开始

### 1. 启动ESP32设备
确保你的ESP32设备已烧录了最新的固件，并且蓝牙功能正常工作。

### 2. 启动Web服务器
```bash
# 方法1: 使用Python脚本（推荐）
python3 serve.py

# 方法2: 使用Python内置服务器
python3 -m http.server 8080

# 方法3: 使用Node.js (如果安装了)
npx http-server -p 8080
```

### 3. 打开浏览器
访问: `http://localhost:8080/web-interface.html`

### 4. 连接设备
1. 点击"连接设备"按钮
2. 在设备列表中选择"ESP32-IR-Recorder"
3. 等待连接成功

## 🔧 浏览器要求

### 支持的浏览器
- ✅ **Chrome** 56+ (推荐)
- ✅ **Edge** 79+
- ✅ **Opera** 43+
- ❌ Firefox (暂不支持Web Bluetooth)
- ❌ Safari (暂不支持Web Bluetooth)

### 系统要求
- **Windows**: Windows 10 1703+ (Creators Update)
- **macOS**: macOS 10.12.6+
- **Linux**: 需要BlueZ 5.41+
- **Android**: Chrome 56+
- **iOS**: 暂不支持

## 📱 使用方法

### 连接设备
1. 确保ESP32设备已启动并处于广播状态
2. 点击"连接设备"按钮
3. 在蓝牙设备列表中选择"ESP32-IR-Recorder"
4. 等待连接成功提示

### 控制LED
- 点击颜色按钮直接控制LED
- 支持的颜色：红色、绿色、蓝色、关闭

### 发送自定义命令
1. 在"自定义命令"输入框中输入命令
2. 点击"发送"按钮或按回车键
3. 支持的命令示例：
   - `red` - 设置红色
   - `green` - 设置绿色  
   - `blue` - 设置蓝色
   - `off` - 关闭LED

### 查看日志
- 实时查看连接状态和数据传输日志
- 支持清空日志和导出日志功能

## 🔧 技术实现

### Web Bluetooth API
使用现代浏览器的Web Bluetooth API实现：
- 设备发现和连接
- GATT服务访问
- 特征值读写操作
- 通知和指示监听

### 与ESP32的通信
- **服务UUID**: `ad91b201-7347-4047-9e17-3bed82d75f9d`
- **接收特征值**: `b6fccb50-87be-44f3-ae22-f85485ea42c4` (用于接收命令)
- **指示特征值**: `503de214-8682-46c4-828f-d59144da41be` (用于接收数据)

### 安全考虑
- 使用HTTPS或localhost访问（Web Bluetooth要求）
- 设备配对和连接管理
- 错误处理和重连机制

## 🐛 故障排除

### 常见问题

#### 1. "浏览器不支持Web Bluetooth API"
- **解决方案**: 使用Chrome或Edge浏览器
- **检查**: 确保浏览器版本足够新

#### 2. "找不到设备"
- **解决方案**: 
  - 确保ESP32设备已启动
  - 检查设备名称是否为"ESP32-IR-Recorder"
  - 尝试重新启动ESP32设备

#### 3. "连接失败"
- **解决方案**:
  - 检查ESP32蓝牙功能是否正常
  - 尝试断开其他设备的连接
  - 重启ESP32设备

#### 4. "发送命令失败"
- **解决方案**:
  - 检查设备连接状态
  - 确保特征值配置正确
  - 查看浏览器控制台错误信息

### 调试技巧
1. 打开浏览器开发者工具 (F12)
2. 查看Console标签页的错误信息
3. 检查Network标签页的连接状态
4. 查看ESP32串口输出日志

## 📝 开发说明

### 文件结构
```
esp-ir-record/
├── web-interface.html    # Web界面主文件
├── serve.py             # HTTP服务器脚本
├── WEB_BLUETOOTH_README.md  # 说明文档
└── src/                 # ESP32源代码
    ├── main.rs
    ├── bluetooth.rs
    └── led.rs
```

### 自定义开发
1. 修改`web-interface.html`中的UUID常量
2. 调整UI样式和功能
3. 添加新的控制命令
4. 扩展日志功能

## 🔮 未来计划

- [ ] 支持更多ESP32功能
- [ ] 添加红外信号录制和回放
- [ ] 支持多设备连接
- [ ] 添加设备配置界面
- [ ] 支持离线模式
- [ ] 添加数据可视化

## 📄 许可证

本项目采用MIT许可证，详见LICENSE文件。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

---

**享受你的ESP32 Web Bluetooth控制体验！** 🚀
