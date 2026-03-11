#!/bin/bash

echo "🚀 股票投资平台 - Dioxus + Charming"
echo "==================================="
echo ""

# 检查 Rust 是否安装
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust 未安装"
    echo "请运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust 版本: $(rustc --version)"
echo ""

# 选择运行模式
echo "请选择运行模式:"
echo "1. 开发模式 (Debug)"
echo "2. 发布模式 (Release - 优化性能)"
echo ""
read -p "请输入选项 (1 或 2): " choice

case $choice in
    1)
        echo ""
        echo "🔨 构建开发版本..."
        cargo build
        
        if [ $? -eq 0 ]; then
            echo ""
            echo "✅ 构建成功！"
            echo "🚀 启动应用..."
            echo ""
            cargo run
        else
            echo ""
            echo "❌ 构建失败"
            exit 1
        fi
        ;;
    2)
        echo ""
        echo "🔨 构建发布版本（可能需要几分钟）..."
        cargo build --release
        
        if [ $? -eq 0 ]; then
            echo ""
            echo "✅ 构建成功！"
            echo "🚀 启动应用..."
            echo ""
            cargo run --release
        else
            echo ""
            echo "❌ 构建失败"
            exit 1
        fi
        ;;
    *)
        echo "❌ 无效选项"
        exit 1
        ;;
esac
