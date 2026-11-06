// SPDX-License-Identifier: GPL-2.0-or-later
//! Connect AppKit's existing About command to the native in-app About page.
use crate::ui::{LocalPage, Message};
use iced::futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    Stream, StreamExt,
};
use objc2::{define_class, msg_send, rc::Retained, runtime::AnyObject, sel, MainThreadOnly};
use objc2_app_kit::NSApplication;
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSObjectProtocol};
use std::{cell::RefCell, sync::Mutex};
static EVENTS: Mutex<Option<UnboundedSender<()>>> = Mutex::new(None);
thread_local! { static TARGET:RefCell<Option<Retained<MenuTarget>>> = const { RefCell::new(None) }; }
define_class!(
    // SAFETY: NSObject has no subclassing requirements; the target lives on the main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    struct MenuTarget;
    unsafe impl NSObjectProtocol for MenuTarget {}
    impl MenuTarget {
        #[unsafe(method(showRippleAbout:))]
        fn show_about(&self, _sender:&AnyObject) {
            if let Some(sender) = EVENTS.lock().unwrap().as_ref() { let _ = sender.unbounded_send(()); }
        }
    }
);
pub fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let Some(menu) = app
        .mainMenu()
        .and_then(|m| m.itemAtIndex(0))
        .and_then(|i| i.submenu())
    else {
        return;
    };
    let Some(about) = menu.itemAtIndex(0) else {
        return;
    };
    about.setTitle(ns_string!("About Ripple"));
    // SAFETY: init is NSObject's initializer, and showRippleAbout: has the menu action signature.
    let target: Retained<MenuTarget> = unsafe { msg_send![MenuTarget::alloc(mtm), init] };
    unsafe {
        about.setTarget(Some(&target));
        about.setAction(Some(sel!(showRippleAbout:)));
    }
    TARGET.with(|slot| *slot.borrow_mut() = Some(target));
}
pub fn events() -> impl Stream<Item = Message> {
    let (sender, receiver) = unbounded();
    *EVENTS.lock().unwrap() = Some(sender);
    receiver.map(|_| Message::Local(LocalPage::About))
}
