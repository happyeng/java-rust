# Java 调用 Rust BDD 库入门指南

## 环境准备

1. 安装 Rust: https://www.rust-lang.org/tools/install
2. 安装 JDK 8+

## 编译 Rust 库

```bash
cd rust-bdd
cargo build --release
```

Windows 生成 `target/release/rust_bdd.dll`，Linux 生成 `target/release/librust_bdd.so`

## 编译 Java

```bash
cd java/src
javac *.java
```

## 运行

将动态库复制到 Java 可执行目录，或设置 `java.library.path`:

```bash
# Windows，进入到java/src目录
cd java/src
java "-Djava.library.path=../../rust-bdd/target/release" Main
```

## 使用示例

```java
RustBdd bdd1 = new RustBdd(3);  // 创建 BDD，3 个变量
RustBdd bdd2 = new RustBdd(3);
RustBdd result = bdd1.and(bdd2);  // AND 运算
result.dispose();  // 释放内存
```

## 项目结构

- `rust-bdd/`: Rust 项目，生成动态库
- `java/src/`: Java 源代码
  - `RustBdd.java`: BDD 封装类
  - `Main.java`: 示例程序

## 核心概念

1. **BDD (Binary Decision Diagram)**: 布尔决策图，用于表示布尔函数
2. **JNI (Java Native Interface)**: Java 调用本地代码的接口
3. **指针传递**: Java 用 `long` 存储 Rust 对象指针
4. **内存管理**: 使用完毕后调用 `dispose()` 释放内存

