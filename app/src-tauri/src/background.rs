//! Keep the work the user started alive when they switch apps (knowledge/11,
//! "Mobile non-negotiables": a reply that is being written, a model that is
//! downloading, a dictation that is running must not die because the user
//! glanced at another app).
//!
//! A `Guard` is held for exactly as long as the work runs and released on
//! drop, whatever path the work takes out. What it does per platform:
//!
//! - **Android**: starts the `VerdureWork` foreground service (a small Kotlin
//!   service written into the generated project by `patch-android-project.py`)
//!   with the kind of work, so the process is neither frozen nor killed while
//!   the app is in the background; stops it when the last guard goes. The
//!   service is reached by an explicit Intent through system classes only, so
//!   no app class has to be found from a native thread.
//! - **iOS**: `UIApplication.beginBackgroundTask`, which buys the app a short
//!   run (the system allows roughly half a minute) after it leaves the
//!   screen. Longer than that iOS suspends the process; the work resumes when
//!   the user returns, it is not lost. Dictation is covered separately by the
//!   `audio` background mode in the app's Info.plist (`build-testflight.yml`).
//! - **macOS**: nothing needed; the class lookup finds no `UIApplication` and
//!   the guard is a no-op, which is also what the tests on this Mac exercise.

/// What the user is waiting on. The Android notification names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Work {
    /// A reply is being generated (or the model it needs is downloading).
    Thinking,
    /// The microphone is open and speech is being transcribed.
    Listening,
}

impl Work {
    /// The word the Android service shows; only Android reads it.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Work::Thinking => "thinking",
            Work::Listening => "listening",
        }
    }
}

/// Held while the work runs; dropping it ends the platform's protection.
#[derive(Debug)]
pub(crate) struct Guard {
    /// Read back on Android, where ending one kind of work must not stop the
    /// service while another kind runs.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) work: Work,
    #[cfg(target_vendor = "apple")]
    task: usize,
}

impl Guard {
    pub(crate) fn begin(work: Work) -> Guard {
        #[cfg(target_os = "android")]
        android::begin(work);
        Guard {
            work,
            #[cfg(target_vendor = "apple")]
            task: apple::begin(),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        android::end(self.work);
        #[cfg(target_vendor = "apple")]
        apple::end(self.task);
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::Work;
    use std::sync::Mutex;

    /// The kinds of work in flight, oldest first. The service shows the
    /// newest; when the last one ends the service stops.
    static ACTIVE: Mutex<Vec<Work>> = Mutex::new(Vec::new());
    const SERVICE: &str = "ai.verdure.porter.VerdureWork";

    pub(super) fn begin(work: Work) {
        let mut active = ACTIVE.lock().unwrap_or_else(|p| p.into_inner());
        active.push(work);
        set(Some(work));
    }

    pub(super) fn end(work: Work) {
        let mut active = ACTIVE.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(i) = active.iter().rposition(|w| *w == work) {
            active.remove(i);
        }
        set(active.last().copied());
    }

    /// Start (or re-describe) the foreground service for `work`, or stop it.
    /// Failures are swallowed after clearing the Java exception: losing
    /// background protection is a degraded run, never a failed reply.
    fn set(work: Option<Work>) {
        use jni::objects::{JObject, JValue};
        let ctx = ndk_context::android_context();
        // SAFETY: the VM pointer and the global Application context were
        // stored by initAudioContext at launch and live for the whole process
        // (same contract as web.rs `network_state`).
        let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else { return };
        let Ok(mut env) = vm.attach_current_thread() else { return };
        let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
        let go = |env: &mut jni::JNIEnv| -> jni::errors::Result<()> {
            let intent = env.new_object("android/content/Intent", "()V", &[])?;
            let class_name: JObject = env.new_string(SERVICE)?.into();
            env.call_method(
                &intent,
                "setClassName",
                "(Landroid/content/Context;Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&context), JValue::Object(&class_name)],
            )?;
            match work {
                Some(w) => {
                    let key: JObject = env.new_string("kind")?.into();
                    let value: JObject = env.new_string(w.name())?.into();
                    env.call_method(
                        &intent,
                        "putExtra",
                        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&key), JValue::Object(&value)],
                    )?;
                    // startForegroundService exists from Android 8 (API 26); the
                    // generated project's minSdk is below it.
                    let sdk = env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()?;
                    let method = if sdk >= 26 { "startForegroundService" } else { "startService" };
                    env.call_method(
                        &context,
                        method,
                        "(Landroid/content/Intent;)Landroid/content/ComponentName;",
                        &[JValue::Object(&intent)],
                    )?;
                }
                None => {
                    env.call_method(&context, "stopService", "(Landroid/content/Intent;)Z", &[JValue::Object(&intent)])?;
                }
            }
            Ok(())
        };
        if go(&mut *env).is_err() {
            let _ = env.exception_clear();
        }
    }
}

#[cfg(target_vendor = "apple")]
mod apple {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use std::ffi::c_void;

    /// `UIBackgroundTaskInvalid`.
    pub(super) const INVALID: usize = usize::MAX;

    /// Ask iOS for background time. On macOS there is no `UIApplication`, so
    /// this is `INVALID` and `end` does nothing — the guard costs nothing on
    /// the desktop and the same code compiles and runs in the Mac's tests.
    pub(super) fn begin() -> usize {
        let Some(cls) = AnyClass::get(c"UIApplication") else { return INVALID };
        // SAFETY: `sharedApplication` and `beginBackgroundTaskWithExpirationHandler:`
        // are documented UIKit selectors; the latter accepts a nil handler and is
        // documented as safe to call from any thread. The returned identifier is
        // an NSUInteger, which is `usize` here.
        unsafe {
            let app: *mut AnyObject = msg_send![cls, sharedApplication];
            if app.is_null() {
                return INVALID;
            }
            let handler: *const c_void = std::ptr::null();
            msg_send![app, beginBackgroundTaskWithExpirationHandler: handler]
        }
    }

    pub(super) fn end(task: usize) {
        if task == INVALID {
            return;
        }
        let Some(cls) = AnyClass::get(c"UIApplication") else { return };
        // SAFETY: as in `begin`; `endBackgroundTask:` takes the identifier
        // `begin` returned, exactly once.
        unsafe {
            let app: *mut AnyObject = msg_send![cls, sharedApplication];
            if app.is_null() {
                return;
            }
            let _: () = msg_send![app, endBackgroundTask: task];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guard_can_be_held_and_dropped_on_this_platform() {
        // On macOS the UIKit lookup finds nothing and the guard is a no-op;
        // on Android and iOS the real calls are exercised on the device. What
        // this proves on every platform: begin/drop never panic, and the
        // guard reports the work it protects.
        let g = Guard::begin(Work::Thinking);
        assert_eq!(g.work, Work::Thinking);
        assert_eq!(Work::Listening.name(), "listening");
        drop(g);
        #[cfg(target_os = "macos")]
        assert_eq!(apple::begin(), apple::INVALID, "no UIApplication on the desktop");
    }
}
