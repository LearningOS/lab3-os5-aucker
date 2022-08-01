## 实验总结

实验环境：Ubuntu20.04
工具：VSCode + Rust-Analyzer

由于之前操作系统相关实践较少，因此在代码编写的过程中遇到很多困难，不过在官方文档和在群友们的帮助下，到目前为止完成了lab3-os5的实验部分，后续还有两个实验需要完成。

## 到目前为止完成的内容

* Rust基础语法学习
* rustlings的练习
* Rust完成的一部分数据结构和算法的实现
* lab0: 基础实验环境的搭建
* lab1: 批处理系统
* lab2: 多道程序与分时多任务
* lab3: 地址空间
* lab4: 进程及进程管理

## Rust学习
* Rust文档丰富，外文文档可以通过rust extension插件搜索
* Rust有着活跃的社区，discord，stackoverflow等社区氛围浓厚
* Rust无GC，通过ownership来保证安全
* Rust借鉴了很多语言的优点，例如函数式，宏，泛型
* 完成了rustlings的学习

## Lab0

安装和配置实验环境。由于Windows下的WSL2配合vscode导致Vemm内存占用过高，云服务器上git不好用，于是采用本地Ubuntu环境。

## Lab1

Lab1目的在于了解系统调用过程。实现一个系统调用`sys_task_info`.
```rust
/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    // -1
    set_task_info(ti);
    0
}
```
通过实验一，我对OS的本质有了一定的认识，同时了解了qemu和真实计算机的启动流程，程序的内存布局和编译过程。
* 将PC寄存器设置为ROM物理地址，运行ROM固件初始化，将bootloader的代码和数据从硬盘载入物理内存中，跳转到bootloader
* 由bootloader加载至物理起始地址0x80000000，进行CPU初始化，并负责将操作系统从硬盘载入物理内存
* 跳转至操作系统，操作系统开始一系列的工作

## Lab2

Lab2的目的在于了解为何要引入虚拟内存机制。实现引入虚拟内存后lab1的系统调用。以及深入了解进程的地址空间和页表。主要实现`sys_mmap`和`sys_unmap`两个系统调用。

由于lab1直接在物理地址上进行实现，所以在程序传过来的指针就是它原本的物理地址。但在引入虚拟内存后，程序传入参数是一个虚拟地址，因此此时应当实现virtaddr到physaddr的转换。

`sys_mmap`的实现思路，首先在调用函数中对传入参数进行合法检测，然后处理权限信息。然后在每个task中提供mmap的接口，由于task的控制块管理着地址空间的信息，因此，在`memory_set`模块中实现最终的`mmap`的功能。

## Lab3 

本实验引入进程的概念，对任务进行抽象。引入进程的一个重要原因是实现隔离。主要实现`sys_spawn`系统调用和实现`stride`调度。

## 总结

非常感谢老师能提供一次系统地学习OS的机会，同时于实践相结合，整个实验整体下来能学到的东西还是很多的，收获很多有用的知识，同时由于第一阶段学习时间较为紧，因此基础知识还需进一步加强巩固。