use super::backend_tests::PythonPage;
use serde_json::json;

struct ShadowingPage;

impl ShadowingPage {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/UploadFinished.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass UploadFinished:\n    note: str\n",
        ),
        (
            "client/services/StampService.py",
            "from models import UploadFinished\n\n\n@service\nclass StampService:\n    done: str = ''\n\n    \
             @on_event\n    def finished(self, finished: UploadFinished) -> None:\n        self.done = finished.note\n",
        ),
    ];
    const DRIVER: &'static str = "_report = {'init': _ask('stamp', {'op': 'init', 'props': {}})}\n\
         _report['heard'] = _ask('stamp', {'op': 'event', 'name': 'finished', 'value': {'note': 'a-7'}})\n\
         _report['done'] = _ask('stamp', {'op': 'get', 'name': 'done'})\n";
}

#[test]
fn a_python_app_model_named_like_a_built_in_event_wins() {
    if !PythonPage::has_python() {
        return;
    }
    let report = PythonPage::run(&ShadowingPage::FILES, &["stamp"], ShadowingPage::DRIVER);
    assert_eq!(report["heard"], json!({"value": null}), "{report}");
    assert_eq!(report["done"], json!({"value": "a-7"}), "{report}");
}
