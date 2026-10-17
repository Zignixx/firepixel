//! Hilfstest: listet Befehle ohne Wirkung und (mit `FP_DUMP=präfix`) die Beschreibung von Befehlen.

#[test]
fn list_unimplemented() {
    let r = fp_engine::CommandRegistry::with_defaults();
    let mut n = 0;
    let dump = std::env::var("FP_DUMP").ok();
    for c in r.iter() {
        let i = c.info();
        if !i.implemented {
            n += 1;
            println!("UNIMPL {} | {} | {:?}", i.name, i.menu_path.join(" > "), i.fw_id);
        }
        if let Some(d) = &dump
            && d.split(',').any(|p| i.name.starts_with(p))
        {
            let ps: Vec<String> = i.params.iter().map(|p| format!("{}{}:{}", p.name, if p.required { "!" } else { "" }, p.description)).collect();
            println!("DUMP {} req={:?} params=[{}] note={:?}", i.name, i.requires, ps.join("; "), i.description);
        }
    }
    println!("TOTAL {} UNIMPL {}", r.len(), n);
}
