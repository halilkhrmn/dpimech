//! State that lives on the UI thread only (Slint models are not `Send`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dpimech_core::model::ProfileState;
use slint::{Image, VecModel};

use crate::AppItem;

thread_local! {
    /// Last profile list received from the service.
    pub static PROFILES: RefCell<Vec<ProfileState>> = const { RefCell::new(Vec::new()) };
    /// Apps of the profile being edited, bound to the editor.
    pub static DRAFT_APPS: Rc<VecModel<AppItem>> = Rc::new(VecModel::default());
    /// Everything the last discovery found, before search filtering.
    pub static PICKER_ALL: RefCell<Vec<AppItem>> = const { RefCell::new(Vec::new()) };
    pub static PICKER_QUERY: RefCell<String> = const { RefCell::new(String::new()) };
    /// Icons by lower-cased routing key, so saved profiles show icons too.
    pub static ICONS: RefCell<HashMap<String, (String, Image)>> = RefCell::new(HashMap::new());
}
