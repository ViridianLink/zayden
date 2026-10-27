const PREFIX: &str = "inline-flex shrink-0 items-center justify-center border text-sm font-medium whitespace-nowrap transition-colors outline-none select-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50 cursor-pointer no-underline";

/// The class list `button_variants` renders, for `kind` in
/// `primary`, `secondary`, `ghost`, `destructive`, optionally suffixed `-lg`.
/// An unknown kind yields a marker no rendered page contains.
pub(crate) fn btn(kind: &str) -> String {
    let (variant, size) = kind
        .strip_suffix("-lg")
        .map_or((kind, "h-9 gap-2 rounded-lg px-4 max-md:h-11"), |variant| {
            (variant, "h-11 gap-2 rounded-lg px-5")
        });
    let variant = match variant {
        "primary" => {
            "border-transparent bg-primary text-primary-foreground shadow-xs hover:bg-primary/90 active:bg-primary/80"
        },
        "secondary" => {
            "border-transparent bg-foreground/5 text-foreground shadow-xs hover:bg-foreground/10 active:bg-foreground/15"
        },
        "ghost" => {
            "border-transparent text-foreground hover:bg-foreground/5 active:bg-foreground/10"
        },
        "destructive" => {
            "border-transparent bg-destructive text-destructive-foreground shadow-xs hover:bg-destructive/90 active:bg-destructive/80"
        },
        other => return format!("unknown-button-kind-{other}"),
    };
    format!("{PREFIX} {variant} {size}")
}
