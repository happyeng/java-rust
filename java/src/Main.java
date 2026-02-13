/**
 * 主程序：演示如何使用 RustBdd 类
 * 创建两个 BDD 对象，执行运算，返回第三个新对象
 */
public class Main {
    public static void main(String[] args) {
        // 创建第一个 BDD 对象，包含 3 个变量
        // 这会调用 Rust 代码创建实际的 BDD 对象
        RustBdd bdd1 = new RustBdd(3);
        
        // 创建第二个 BDD 对象，同样包含 3 个变量
        RustBdd bdd2 = new RustBdd(3);

        // 对两个 BDD 执行 AND 运算
        // 这会调用 Rust 代码执行运算，返回一个新的 BDD 对象
        RustBdd result = bdd1.and(bdd2);

        // 打印三个 BDD 对象的指针值（用于验证对象已创建）
        System.out.println("BDD1: " + bdd1.getPtr());
        System.out.println("BDD2: " + bdd2.getPtr());
        System.out.println("Result: " + result.getPtr());

        // 释放所有 BDD 对象的内存
        // 这是必须的，否则会造成内存泄漏
        bdd1.dispose();
        bdd2.dispose();
        result.dispose();

        System.out.println("BDD1: " + bdd1.getPtr());
        System.out.println("BDD2: " + bdd2.getPtr());
        System.out.println("Result: " + result.getPtr());
    }
}

