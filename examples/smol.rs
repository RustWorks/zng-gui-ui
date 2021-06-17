use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let size = var(0);
        let file = "examples/res/icon-bytes.png";        
        window! {
            title = "`smol` example";
            font_family = "monospace";
            content = v_stack(widgets![
                text(formatx!(r#"> using `smol` to read "{}".."#, file)),
                text(size.map(|&i| if i == 0 { "".to_text() } else { formatx!("> done, {} bytes", i) }))
            ]);
            on_open = move |ctx, _| {
                let size = ctx.vars.sender(&size);
                Tasks::run(async move {
                    // `smol` starts their own *event reactor* so we can just start using async IO functions:
                    let bytes = smol::fs::read(file).await.unwrap();
                    
                    let _ = size.send(bytes.len());
                })
            };
        }
    })
}
