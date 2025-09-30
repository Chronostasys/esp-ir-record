#!/usr/bin/env python3
"""
简单的HTTP服务器，用于托管ESP32 Web Bluetooth控制界面
"""

import http.server
import socketserver
import webbrowser
import os
import sys
from pathlib import Path

def main():
    # 设置端口
    PORT = 8080
    
    # 切换到脚本所在目录
    script_dir = Path(__file__).parent
    os.chdir(script_dir)
    
    # 创建HTTP服务器
    Handler = http.server.SimpleHTTPRequestHandler
    
    try:
        with socketserver.TCPServer(("", PORT), Handler) as httpd:
            print(f"🚀 ESP32 Web Bluetooth 控制台已启动!")
            print(f"📱 访问地址: http://localhost:{PORT}/web-interface.html")
            print(f"📁 服务目录: {script_dir}")
            print(f"🔵 确保你的ESP32设备已启动并处于广播状态")
            print(f"💡 提示: 在Chrome/Edge浏览器中打开上述地址")
            print(f"⏹️  按 Ctrl+C 停止服务器")
            print("-" * 60)
            
            # 自动打开浏览器
            try:
                webbrowser.open(f'http://localhost:{PORT}/web-interface.html')
            except:
                pass
            
            # 启动服务器
            httpd.serve_forever()
            
    except KeyboardInterrupt:
        print(f"\n🛑 服务器已停止")
    except OSError as e:
        if e.errno == 48:  # Address already in use
            print(f"❌ 端口 {PORT} 已被占用，请尝试其他端口")
            print(f"💡 可以修改脚本中的 PORT 变量")
        else:
            print(f"❌ 服务器启动失败: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
