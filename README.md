# Ren'Rs

一款你的下一代视觉小说引擎！

编译器和运行器分开处理！每一个 .rrs 都是一个完整的游戏！

Lua 教程可以参考 [官方教程 gitbook](https://candysharkstudio.gitbook.io/ren-rs-lua-tutorial)，**需要挂七根木棍才能上**

请注意！使用本教程的 Lua 已经可以解决 99% 的 UI/UX 问题，视觉小说完全可以直接用这个做！还有 1% 的问题是可能实在是引擎部分解决不了的，需要手动修改 Ren'Rs 的源代码的。。

# 开源协议

以 Apache 2.0 协议开放源代码！各位仅需在【帮助】页鸣谢一下原作者即可！我同样允许各位以闭源形式发布各位的视觉小说！

# 交叉编译


# 鸣谢

1. [xphost](https://github.com/xphost008)：框架开发者
2. [小万泥](https://github.com/FireDragon0659)：框架开发者

# 特殊鸣谢

1. [Vite](https://github.com/vitejs/vite)
2. [TypeScript](https://github.com/Microsoft/TypeScript)
3. [Lit](https://github.com/lit/lit)
4. [Rust](https://github.com/rust-lang/rust)
5. [Tauri](https://github.com/tauri-apps/tauri)

# 使用事宜

1. 本框架采用 Vite + Vanilla 打包！部分组件用了 Lit 库，使用了 TypeScript。 
2. 除此之外，并未使用任何前端框架，使用纯 Vanilla 库进行构建并打包，部分 rainbow、scare 等标签使用了 Lit。
3. Lit 是一个用于构建原生 HTML 标签的一个框架！它非常好用，我很喜欢！

> [!TIPS] 重要
> 本引擎目前暂时不支持交叉编译（其实就是 Tauri 官方的问题！！），各位可以用咱们的官方 Github Actions 去跨平台编译哦！
