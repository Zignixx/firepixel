//! JS-Laufzeit: Kontext mit globalen Objekten (`fw`, `Files`, `Errors`, `Tools`, `System`, `$`,
//! `alert` …), Prototypen je Klasse aus der Bindungstabelle, Wrapper-Objekte mit Handle,
//! Aufrufsemantik nach 03 §10.3 (Vorgaben, EBadParamCount, überzählige Argumente ignoriert,
//! Typprüfung) und Ausnahmen nach §10.4 (HRESULT als Zahl).

use crate::bindings::{self, CLASSES, DefaultArg, hr};
use crate::conv::{self, Arg, HANDLE_KEY, convert_arg};
use crate::imp;
use crate::state::{FwErr, Handle, with_state};
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsProxyBuilder;
use boa_engine::property::{Attribute, PropertyDescriptor};
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction,
    js_string,
};

/// Name des globalen Hilfsobjekts mit den Prototypen.
const PROTOS: &str = "__fp_protos";

/// Wandelt einen API-Fehler in eine JS-Ausnahme.
pub fn throw(e: FwErr, member: &str) -> JsError {
    match e {
        FwErr::Hr(code, msg) => {
            let _ = with_state(|st| {
                st.diagnostics
                    .push(format!("{member}: {msg} (0x{code:08x})"))
            });
            JsError::from_opaque(JsValue::from(f64::from(code as i32)))
        }
        FwErr::NotImpl(what) => {
            let _ = with_state(|st| {
                st.not_implemented.insert(what.clone());
            });
            JsNativeError::error()
                .with_message(fp_i18n::tr!("err-script-not-implemented", member = what))
                .into()
        }
        FwErr::Js(m) => JsNativeError::error().with_message(m).into(),
    }
}

fn js_err(e: JsError) -> FwErr {
    FwErr::Js(e.to_string())
}

/// Neuer Kontext mit allen globalen Objekten.
pub fn new_context(loop_limit: u64) -> JsResult<Context> {
    let mut ctx = Context::default();
    ctx.runtime_limits_mut()
        .set_loop_iteration_limit(loop_limit);
    ctx.runtime_limits_mut().set_recursion_limit(2048);
    let holder = JsObject::with_null_proto();
    ctx.register_global_property(js_string!("__fp_protos"), holder, Attribute::empty())?;
    // FwArray: Klasse für gelesene Listen (instanceof FwArray)
    let fwarray = FunctionObjectBuilder::new(
        ctx.realm(),
        NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
    )
    .name(js_string!("FwArray"))
    .constructor(true)
    .build();
    let fproto = JsObject::with_object_proto(ctx.intrinsics());
    fwarray.define_property_or_throw(
        js_string!("prototype"),
        PropertyDescriptor::builder()
            .value(fproto.clone())
            .writable(false)
            .enumerable(false)
            .configurable(false),
        &mut ctx,
    )?;
    fproto.define_property_or_throw(
        js_string!("constructor"),
        PropertyDescriptor::builder()
            .value(fwarray.clone())
            .writable(true)
            .enumerable(false)
            .configurable(true),
        &mut ctx,
    )?;
    ctx.register_global_property(
        js_string!("FwArray"),
        fwarray,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    )?;
    // Einzelobjekte
    for (global, class) in [
        ("fw", "Firepixel"),
        ("firepixel", "Firepixel"),
        ("Files", "Files"),
        ("Errors", "Errors"),
        ("Tools", "Tools"),
        ("System", "System"),
        ("$", "$"),
        ("Document", "DocumentStatic"),
        ("Widget", "Widget"),
    ] {
        let v = wrap(&mut ctx, class, Handle::Singleton)
            .map_err(|e| JsNativeError::error().with_message(format!("{e:?}")))?;
        ctx.register_global_property(
            JsString::from(global),
            v,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )?;
    }
    // app (Common API): Alias auf fw
    let fw = ctx.global_object().get(js_string!("fw"), &mut ctx)?;
    ctx.register_global_property(
        js_string!("app"),
        fw.clone(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    )?;
    // Kompatibilität mit Skripten des Originals: dessen Programmname (Haupt-EXE im Ressourcenordner,
    // `fp_color::install`) als weiterer globaler Alias und `create<Name>Document` als Alias von
    // `createFirepixelDocument`. Ohne Ressourcenordner entfallen die Aliase (bewusste Abweichung).
    if let Some(name) = fp_color::install::original_app_name() {
        let alias = JsString::from(name.to_lowercase());
        if !ctx.global_object().has_property(alias.clone(), &mut ctx)? {
            ctx.register_global_property(alias, fw, Attribute::WRITABLE | Attribute::CONFIGURABLE)?;
        }
        let proto = proto_for("Firepixel", &mut ctx)?;
        let create = proto.get(js_string!("createFirepixelDocument"), &mut ctx)?;
        let key = JsString::from(format!("create{name}Document"));
        if !create.is_undefined() && !proto.has_own_property(key.clone(), &mut ctx)? {
            proto.define_property_or_throw(
                key,
                PropertyDescriptor::builder()
                    .value(create)
                    .writable(true)
                    .enumerable(false)
                    .configurable(true),
                &mut ctx,
            )?;
        }
    }
    // globale Funktionen
    let globals: &[(&str, boa_engine::native_function::NativeFunctionPointer)] = &[
        ("alert", imp::global_alert),
        ("confirm", imp::global_confirm),
        ("prompt", imp::global_prompt),
        ("write", imp::global_write),
        ("writeLog", imp::global_write),
        ("WRITE_HTML", imp::global_write),
        ("WRITE_CSS", imp::global_write),
        ("WRITE_FXG", imp::global_write),
        ("WRITE_EXTERNAL_CSS", imp::global_write),
        ("mb_msg", imp::global_alert),
        ("localize", imp::global_localize),
        ("garbageCollect", imp::global_noop),
        ("uneval", imp::global_uneval),
    ];
    for (name, f) in globals {
        ctx.register_global_callable(JsString::from(*name), 0, NativeFunction::from_fn_ptr(*f))?;
    }
    // Klassen als globale Konstruktoren für `instanceof` (z. B. `e instanceof Text`); Unterklassen
    // erben vom Prototyp der Basisklasse
    for cls in [
        "Element",
        "Path",
        "Text",
        "Image",
        "Group",
        "Instance",
        "RectanglePrimitive",
        "Hotspot",
        "SliceHotspot",
        "CompoundShape",
        "Layer",
        "Frame",
        "Contour",
        "ContourNode",
    ] {
        let proto = proto_for(cls, &mut ctx)?;
        if let Some(parent) = bindings::parents(cls).iter().find(|p| !p.starts_with('(')) {
            let pp = proto_for(parent, &mut ctx)?;
            proto.set_prototype(Some(pp));
        }
        let ctor = FunctionObjectBuilder::new(
            ctx.realm(),
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
        )
        .name(JsString::from(cls))
        .constructor(true)
        .build();
        ctor.define_property_or_throw(
            js_string!("prototype"),
            PropertyDescriptor::builder()
                .value(proto.clone())
                .writable(false)
                .enumerable(false)
                .configurable(false),
            &mut ctx,
        )?;
        if !ctx
            .global_object()
            .has_property(JsString::from(cls), &mut ctx)?
        {
            ctx.register_global_property(
                JsString::from(cls),
                ctor,
                Attribute::WRITABLE | Attribute::CONFIGURABLE,
            )?;
        }
    }
    // „App“: weiterer Alias auf fw (Skripte des Originals)
    let fw_obj = ctx.global_object().get(js_string!("fw"), &mut ctx)?;
    ctx.register_global_property(
        js_string!("App"),
        fw_obj,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    )?;
    // fp: direkter Zugang zum Befehlsregister (gespeicherte Befehle, „Schritte kopieren“)
    let fp = JsObject::with_object_proto(ctx.intrinsics());
    for (name, f) in [
        (
            "run",
            imp::global_fp_run as boa_engine::native_function::NativeFunctionPointer,
        ),
        ("available", imp::global_fp_available),
        ("saveAs", imp::global_fp_save_as),
        ("t", imp::global_fp_t),
    ] {
        let func = FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(f))
            .name(JsString::from(name))
            .length(2)
            .build();
        fp.define_property_or_throw(
            JsString::from(name),
            PropertyDescriptor::builder()
                .value(func)
                .writable(true)
                .enumerable(false)
                .configurable(true),
            &mut ctx,
        )?;
    }
    ctx.register_global_property(
        js_string!("fp"),
        fp,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    )?;
    // toSource (SpiderMonkey-Erweiterung) für Objekte und Arrays
    let ts = FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(imp::to_source))
        .name(js_string!("toSource"))
        .build();
    let obj_proto = ctx.intrinsics().constructors().object().prototype();
    obj_proto.define_property_or_throw(
        js_string!("toSource"),
        PropertyDescriptor::builder()
            .value(ts)
            .writable(true)
            .enumerable(false)
            .configurable(true),
        &mut ctx,
    )?;
    Ok(ctx)
}

/// Prototyp für eine Klasse (bei Bedarf gebaut und zwischengespeichert).
pub fn proto_for(js: &str, ctx: &mut Context) -> JsResult<JsObject> {
    let holder = ctx.global_object().get(js_string!("__fp_protos"), ctx)?;
    let holder = holder
        .as_object()
        .ok_or_else(|| JsNativeError::error().with_message(PROTOS))?;
    let key = JsString::from(js);
    let existing = holder.get(key.clone(), ctx)?;
    if let Some(o) = existing.as_object() {
        return Ok(o);
    }
    let proto = JsObject::with_object_proto(ctx.intrinsics());
    proto.define_property_or_throw(
        js_string!("__fpc"),
        PropertyDescriptor::builder()
            .value(JsString::from(js))
            .writable(false)
            .enumerable(false)
            .configurable(false),
        ctx,
    )?;
    let ts =
        FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(wrapper_to_string))
            .name(js_string!("toString"))
            .build();
    proto.define_property_or_throw(
        js_string!("toString"),
        PropertyDescriptor::builder()
            .value(ts)
            .writable(true)
            .enumerable(false)
            .configurable(true),
        ctx,
    )?;
    let mut seen_m: Vec<&str> = Vec::new();
    let mut seen_p: Vec<&str> = Vec::new();
    for c in bindings::chain(js) {
        let ci = CLASSES.iter().position(|x| std::ptr::eq(x, c)).unwrap_or(0) as u32;
        for (mi, m) in c.methods.iter().enumerate() {
            if seen_m.contains(&m.name) {
                continue;
            }
            seen_m.push(m.name);
            let f = NativeFunction::from_copy_closure_with_captures(
                |this, args, caps: &(u32, u32), ctx| method_entry(this, args, *caps, ctx),
                (ci, mi as u32),
            );
            let func = FunctionObjectBuilder::new(ctx.realm(), f)
                .name(JsString::from(m.name))
                .length(0)
                .build();
            proto.define_property_or_throw(
                JsString::from(m.name),
                PropertyDescriptor::builder()
                    .value(func)
                    .writable(true)
                    .enumerable(false)
                    .configurable(true),
                ctx,
            )?;
        }
        for (pi, p) in c.props.iter().enumerate() {
            if seen_p.contains(&p.name) || seen_m.contains(&p.name) {
                continue;
            }
            seen_p.push(p.name);
            let g = NativeFunction::from_copy_closure_with_captures(
                |this, _args, caps: &(u32, u32), ctx| getter_entry(this, *caps, ctx),
                (ci, pi as u32),
            );
            let getter = FunctionObjectBuilder::new(ctx.realm(), g)
                .name(JsString::from(format!("get {}", p.name)))
                .build();
            let setter = if p.readonly {
                None
            } else {
                let sf = NativeFunction::from_copy_closure_with_captures(
                    |this, args, caps: &(u32, u32), ctx| setter_entry(this, args, *caps, ctx),
                    (ci, pi as u32),
                );
                Some(
                    FunctionObjectBuilder::new(ctx.realm(), sf)
                        .name(JsString::from(format!("set {}", p.name)))
                        .build(),
                )
            };
            proto.define_property_or_throw(
                JsString::from(p.name),
                PropertyDescriptor::builder()
                    .get(getter)
                    .maybe_set(setter)
                    .enumerable(true)
                    .configurable(true),
                ctx,
            )?;
        }
    }
    // Firepixel-own methods (`bindings::EXT_METHODS`), addressed with the class index `EXT_CLASS`
    for (ei, (cls, m)) in bindings::EXT_METHODS.iter().enumerate() {
        if *cls != js || seen_m.contains(&m.name) {
            continue;
        }
        seen_m.push(m.name);
        let f = NativeFunction::from_copy_closure_with_captures(
            |this, args, caps: &(u32, u32), ctx| method_entry(this, args, *caps, ctx),
            (EXT_CLASS, ei as u32),
        );
        let func = FunctionObjectBuilder::new(ctx.realm(), f)
            .name(JsString::from(m.name))
            .length(0)
            .build();
        proto.define_property_or_throw(
            JsString::from(m.name),
            PropertyDescriptor::builder()
                .value(func)
                .writable(true)
                .enumerable(false)
                .configurable(true),
            ctx,
        )?;
    }
    holder.create_data_property_or_throw(key, proto.clone(), ctx)?;
    Ok(proto)
}

/// Class index of a Firepixel-own method (`bindings::EXT_METHODS`) in the method captures.
const EXT_CLASS: u32 = u32::MAX;

fn wrapper_to_string(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = match this.as_object() {
        Some(o) => o
            .get(js_string!("__fpc"), ctx)?
            .as_string()
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "Object".into()),
        None => "Object".into(),
    };
    Ok(conv::s(&format!("[object {name}]")))
}

/// Neues Wrapper-Objekt der Klasse `js` für `h`.
pub fn wrap(ctx: &mut Context, js: &str, h: Handle) -> Result<JsValue, FwErr> {
    let idx = with_state(|st| st.push_handle(h))?;
    let proto = proto_for(js, ctx).map_err(js_err)?;
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    obj.set_prototype(Some(proto));
    obj.define_property_or_throw(
        JsString::from(HANDLE_KEY),
        PropertyDescriptor::builder()
            .value(idx as f64)
            .writable(false)
            .enumerable(false)
            .configurable(true),
        ctx,
    )
    .map_err(js_err)?;
    Ok(obj.into())
}

/// Lebende Sammlung bzw. lebendes Wörterbuch als Proxy (Knoten, Konturen, customData).
pub fn wrap_proxy(ctx: &mut Context, h: Handle, list: bool) -> Result<JsValue, FwErr> {
    let idx = with_state(|st| st.push_handle(h))?;
    let target = JsObject::with_object_proto(ctx.intrinsics());
    if list {
        let ctor = ctx
            .global_object()
            .get(js_string!("FwArray"), ctx)
            .map_err(js_err)?;
        if let Some(c) = ctor.as_object() {
            let p = c.get(js_string!("prototype"), ctx).map_err(js_err)?;
            if let Some(p) = p.as_object() {
                target.set_prototype(Some(p));
            }
        }
    }
    target
        .define_property_or_throw(
            JsString::from(HANDLE_KEY),
            PropertyDescriptor::builder()
                .value(idx as f64)
                .writable(false)
                .enumerable(false)
                .configurable(true),
            ctx,
        )
        .map_err(js_err)?;
    let p = JsProxyBuilder::new(target)
        .get(imp::proxy_get)
        .set(imp::proxy_set)
        .has(imp::proxy_has)
        .own_keys(imp::proxy_own_keys)
        .get_own_property_descriptor(imp::proxy_get_own_property_descriptor)
        .build(ctx)
        .map_err(js_err)?;
    Ok(p.into())
}

/// Handle eines Objekts (Wrapper oder Proxy-Ziel).
pub fn handle(v: &JsValue, ctx: &mut Context) -> Option<(usize, Handle)> {
    let i = conv::handle_of(v, ctx)?;
    with_state(|st| st.handles.get(i).cloned())
        .ok()
        .flatten()
        .map(|h| (i, h))
}

/// Klassenname eines Wrappers (`__fpc` des Prototyps).
pub fn class_of(v: &JsValue, ctx: &mut Context) -> String {
    v.as_object()
        .and_then(|o| o.get(js_string!("__fpc"), ctx).ok())
        .and_then(|x| x.as_string().map(|s| s.to_std_string_escaped()))
        .unwrap_or_default()
}

fn method_entry(
    this: &JsValue,
    args: &[JsValue],
    caps: (u32, u32),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let (cls_js, m) = if caps.0 == EXT_CLASS {
        let Some((c, m)) = bindings::EXT_METHODS.get(caps.1 as usize) else {
            return Ok(JsValue::undefined());
        };
        (*c, m)
    } else {
        let Some(cls) = CLASSES.get(caps.0 as usize) else {
            return Ok(JsValue::undefined());
        };
        let Some(m) = cls.methods.get(caps.1 as usize) else {
            return Ok(JsValue::undefined());
        };
        (cls.js, m)
    };
    let member = format!("{}.{}", cls_js, m.name);
    let _ = with_state(|st| {
        st.used.insert(member.clone());
    });
    let mut out: Vec<Arg> = Vec::with_capacity(m.params.len());
    for (i, p) in m.params.iter().enumerate() {
        if let Some(v) = args.get(i) {
            // undefined am Ende zählt im Original als vorhandenes Argument; wie fehlend behandeln,
            // wenn es eine Vorgabe gibt
            if v.is_undefined() && p.default != DefaultArg::Req {
                out.push(default_arg(p.default));
                continue;
            }
            out.push(convert_arg(v, p.code, ctx).map_err(|e| throw(e, &member))?);
        } else {
            match p.default {
                DefaultArg::Req => {
                    return Err(throw(
                        FwErr::Hr(
                            hr::E_BAD_PARAM_COUNT,
                            format!("Pflichtparameter „{}“ fehlt", p.name),
                        ),
                        &member,
                    ));
                }
                d => out.push(default_arg(d)),
            }
        }
    }
    let h = handle(this, ctx).map(|x| x.1).unwrap_or(Handle::Singleton);
    imp::call_method(cls_js, m.name, &h, &out, this, ctx).map_err(|e| throw(e, &member))
}

fn default_arg(d: DefaultArg) -> Arg {
    match d {
        DefaultArg::Req | DefaultArg::Null => Arg::Null,
        DefaultArg::Bool(b) => Arg::Bool(b),
        DefaultArg::Int(i) => Arg::Num(i as f64),
    }
}

fn getter_entry(this: &JsValue, caps: (u32, u32), ctx: &mut Context) -> JsResult<JsValue> {
    let Some(cls) = CLASSES.get(caps.0 as usize) else {
        return Ok(JsValue::undefined());
    };
    let Some(p) = cls.props.get(caps.1 as usize) else {
        return Ok(JsValue::undefined());
    };
    let member = format!("{}.{}", cls.js, p.name);
    let _ = with_state(|st| {
        st.used.insert(member.clone());
    });
    let h = handle(this, ctx).map(|x| x.1).unwrap_or(Handle::Singleton);
    imp::get_prop(cls.js, p.name, &h, this, ctx).map_err(|e| throw(e, &member))
}

fn setter_entry(
    this: &JsValue,
    args: &[JsValue],
    caps: (u32, u32),
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let Some(cls) = CLASSES.get(caps.0 as usize) else {
        return Ok(JsValue::undefined());
    };
    let Some(p) = cls.props.get(caps.1 as usize) else {
        return Ok(JsValue::undefined());
    };
    let member = format!("{}.{} =", cls.js, p.name);
    let _ = with_state(|st| {
        st.used.insert(member.clone());
    });
    let v = args.first().cloned().unwrap_or_default();
    let a = convert_arg(&v, p.code, ctx).map_err(|e| throw(e, &member))?;
    let h = handle(this, ctx).map(|x| x.1).unwrap_or(Handle::Singleton);
    imp::set_prop(cls.js, p.name, &h, a, &v, this, ctx).map_err(|e| throw(e, &member))?;
    Ok(JsValue::undefined())
}
