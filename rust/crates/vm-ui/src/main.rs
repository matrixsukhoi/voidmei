//! iced 软件渲染可行性验证 (D1): 无 GPU 上下文打开窗口
//! 注: view/update 用具名函数 (闭包会触发高阶生命周期推断失败)

fn update(_state: &mut (), _message: ()) {}

fn view(_state: &()) -> iced::Element<'_, ()> {
    iced::widget::text("VoidMei Rust MainForm (iced tiny-skia)").into()
}

fn main() -> iced::Result {
    println!("vm-ui: iced tiny-skia 冒烟 (窗口打开由人工验证)");
    iced::application::<(), (), iced::Theme, iced::Renderer>(
        "VoidMei 设置 — iced 冒烟",
        update,
        view,
    )
    .theme(|_| iced::Theme::default())
    .window(iced::window::Settings {
        size: iced::Size::new(400.0, 200.0),
        ..Default::default()
    })
    .run()
}
