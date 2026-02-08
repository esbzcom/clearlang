use clg_ir::TrapCode;
use wasmtime as wt;

use super::wasm_state::set_caller_global_i32;

pub(super) struct RuntimeErrorDiag {
    pub(super) code: &'static str,
    pub(super) message: String,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) detail: Option<String>,
}

pub(super) fn set_runtime_error<T>(caller: &mut wt::Caller<'_, T>, code: TrapCode) {
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_code", code.as_i32());
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_start", 0);
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_end", 0);
    let _ = set_caller_global_i32(caller, "__clg_runtime_error_detail", 0);
}

pub(super) fn extract_runtime_error<T>(
    instance: &wt::Instance,
    store: &mut wt::Store<T>,
) -> Option<RuntimeErrorDiag> {
    let code = get_global(instance, store, "__clg_runtime_error_code")?;
    if code == 0 {
        return None;
    }

    let mut start = get_global(instance, store, "__clg_runtime_error_start").unwrap_or(0) as usize;
    let mut end = get_global(instance, store, "__clg_runtime_error_end").unwrap_or(0) as usize;
    let detail = get_global(instance, store, "__clg_runtime_error_detail").unwrap_or(0);

    let (code_label, mut message, detail_label, reset_span) = match code {
        1 => {
            let label = match detail {
                1 => "ensure",
                _ => "require",
            };
            (
                "R000",
                format!("contract `{}` guard failed", label),
                Some(label.to_string()),
                false,
            )
        }
        2 => (
            "R001",
            "string allocator ran out of memory".to_string(),
            None,
            false,
        ),
        3 => (
            "R002",
            "runtime rejected invalid input".to_string(),
            None,
            false,
        ),
        4 => {
            let kind = match detail {
                1 => "Result",
                2 => "Enum",
                _ => "Option",
            };
            let tag = start;
            (
                "R003",
                format!("{kind} variant observed invalid tag {}", tag),
                Some(kind.to_string()),
                true,
            )
        }
        5 => ("R004", "runtime limits exceeded".to_string(), None, false),
        6 => ("R005", "unsigned integer overflow".to_string(), None, false),
        7 => (
            "R006",
            "crypto algorithm unsupported or unknown".to_string(),
            None,
            false,
        ),
        8 => (
            "R007",
            "crypto input length is invalid for selected algorithm".to_string(),
            None,
            false,
        ),
        9 => ("R008", "crypto input is malformed".to_string(), None, false),
        10 => ("R009", "collection bounds error".to_string(), None, false),
        11 => ("R010", "invalid collection handle".to_string(), None, false),
        _ => (
            "R999",
            format!("runtime trap with unknown code {}", code),
            None,
            false,
        ),
    };

    if reset_span {
        start = 0;
        end = 0;
    }

    if code_label == "R000" && start == 0 && end == 0 {
        message.push_str(" (no span available)");
    }

    Some(RuntimeErrorDiag {
        code: code_label,
        message,
        start,
        end,
        detail: detail_label,
    })
}

fn get_global<T>(instance: &wt::Instance, store: &mut wt::Store<T>, name: &str) -> Option<i32> {
    let global = instance.get_global(&mut *store, name)?;
    global.get(&mut *store).i32()
}

pub(super) fn extract_wasmtime_limit_error(trap: &anyhow::Error) -> Option<RuntimeErrorDiag> {
    let trap = trap
        .chain()
        .find_map(|err| err.downcast_ref::<wt::Trap>())?;
    match trap {
        wt::Trap::OutOfFuel | wt::Trap::Interrupt => Some(RuntimeErrorDiag {
            code: "R004",
            message: "runtime limits exceeded".to_string(),
            start: 0,
            end: 0,
            detail: None,
        }),
        _ => None,
    }
}
