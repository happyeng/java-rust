import java.io.File;

/**
 * RustBdd 类：Java 端对 Rust BDD 库的封装
 * 通过 JNI 调用 Rust 实现的 BDD 操作
 */
public class RustBdd {
    // 静态初始化块：在类加载时自动执行，用于加载 Rust 编译的动态库
    static {
        try {
            String libraryPath = System.getProperty("java.library.path");
            if (libraryPath != null && !libraryPath.isEmpty()) {
                File libDir = new File(libraryPath);
                File dllFile = new File(libDir, "rust_bdd.dll");
                String absPath = dllFile.getAbsolutePath();
                if (dllFile.exists()) {
                    System.load(absPath);
                } else {
                    throw new UnsatisfiedLinkError("DLL 文件不存在: " + absPath);
                }
            } else {
                System.loadLibrary("rust_bdd");
            }
        } catch (Exception e) {
            e.printStackTrace();
            throw new UnsatisfiedLinkError("无法加载 rust_bdd 动态库: " + e.getMessage());
        }
    }

    // 存储 Rust 中 BDD 对象的指针（在 Rust 中是 BddWrapper 的地址）
    // Java 用 long 类型存储 64 位指针值
    private long ptr;
    // 未使用的标志常量（保留用于未来扩展）
    private static final long FROM_PTR_FLAG = -1;

    /**
     * 公共构造函数：创建新的 BDD 对象
     * @param varCount BDD 变量的数量
     */
    public RustBdd(long varCount) {
        // 调用 native 方法创建 BDD，返回 Rust 中对象的指针
        this.ptr = createBdd(varCount);
    }

    /**
     * 私有构造函数：从已有指针创建 RustBdd 对象
     * 用于运算结果，不创建新的 Rust 对象，只是包装已有指针
     * @param ptr Rust 中 BDD 对象的指针
     * @param fromPtr 标志位，用于区分公共构造函数（未使用但保留以区分构造函数签名）
     */
    private RustBdd(long ptr, boolean fromPtr) {
        this.ptr = ptr;
    }

    // ========== JNI Native 方法声明 ==========
    // JNI（Java Native Interface）是 Java 与本地代码（如 C/C++/Rust）互相调用的接口标准。
    // native 关键字在 Java 中用于声明，这些方法不在 Java 里面实现，而是由本地代码（通过 JNI）在如 Rust 的 lib.rs 文件中实现。
    // lib.rs 是 Rust 工程的主库入口文件，也是所有本地实现代码的登记和暴露位置。
    //
    // JNI 的标准使用流程如下：
    // 1. 在 Java 中用 native 关键字声明本地方法。
    // 2. 编译 Java 文件，生成对应的 .class 文件。
    // 3. 用 javac 和 javah（或 javac -h）生成 JNI 的 C 头文件。
    // 4. 在本地语言（如 C/C++/Rust）的源文件（如 Rust 的 lib.rs）中，实现这些 JNI 方法接口。
    // 5. 编译生成动态库（如 .dll, .so, .dylib 等）。
    // 这一步的作用是把本地代码（例如 Rust 项目的 lib.rs 及相关文件）编译成操作系统可加载的动态链接库文件（如 Windows 下的 .dll，Linux 下的 .so，macOS 下的 .dylib），
    // 以便 Java 程序在运行时通过 System.loadLibrary 或 System.load 加载并调用里面实现的本地方法。
    // 6. 在 Java 程序中通过 System.loadLibrary 或 System.load 加载本地动态库。
    // 7. Java 调用 native 方法时，就会通过 JNI 桥接到你在本地实现的业务逻辑。
    
    /**
     * 在 Rust 中创建新的 BDD 对象
     * @param varCount 变量数量
     * @return Rust 中 BDD 对象的指针
     */
    // 这一行声明了一个native方法，对应于Rust里用JNI暴露出来的createBdd函数
    public native long createBdd(long varCount);
    
    /**
     * 对两个 BDD 执行 AND 运算
     * @param ptr1 第一个 BDD 的指针
     * @param ptr2 第二个 BDD 的指针
     * @return 运算结果 BDD 的指针
     */
    public native long andBdd(long ptr1, long ptr2);
    
    /**
     * 对两个 BDD 执行 OR 运算
     * @param ptr1 第一个 BDD 的指针
     * @param ptr2 第二个 BDD 的指针
     * @return 运算结果 BDD 的指针
     */
    public native long orBdd(long ptr1, long ptr2);
    
    /**
     * 释放 Rust 中的 BDD 对象，避免内存泄漏
     * @param ptr 要释放的 BDD 对象指针
     */
    public native void freeBdd(long ptr);

    /**
     * 获取当前 BDD 对象的指针（主要用于调试）
     * @return Rust 中 BDD 对象的指针
     */
    public long getPtr() {
        return ptr;
    }

    /**
     * 对当前 BDD 和另一个 BDD 执行 AND 运算
     * @param other 另一个 BDD 对象
     * @return 新的 RustBdd 对象，包含运算结果
     */
    public RustBdd and(RustBdd other) {
        // 调用 native 方法执行 AND 运算，返回结果 BDD 的指针
        long resultPtr = andBdd(this.ptr, other.ptr);
        // 使用私有构造函数创建新的 Java 对象包装结果指针
        return new RustBdd(resultPtr, true);
    }

    /**
     * 对当前 BDD 和另一个 BDD 执行 OR 运算
     * @param other 另一个 BDD 对象
     * @return 新的 RustBdd 对象，包含运算结果
     */
    public RustBdd or(RustBdd other) {
        // 调用 native 方法执行 OR 运算
        long resultPtr = orBdd(this.ptr, other.ptr);
        // 包装结果指针
        return new RustBdd(resultPtr, true);
    }

    /**
     * 释放 Rust 中的 BDD 对象内存
     * 使用完 BDD 对象后必须调用此方法，否则会造成内存泄漏
     */
    public void dispose() {
        if (ptr != 0) {
            // 调用 native 方法释放 Rust 中的内存
            freeBdd(ptr);
            // 将指针置为 0，防止重复释放
            ptr = 0;
        }
    }
}

