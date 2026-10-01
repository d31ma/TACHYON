#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct WindowControls {
    scope: std::rc::Rc<NativeCapabilityScope>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for WindowControls {
    fn default() -> Self {
        Self {
            scope: NativeCapabilities::scope(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WindowControls {
    pub fn granted(&self, control: &str) -> Result<bool, String> {
        let answer = self.scope.call(
            "window.granted",
            vec![(String::from("control"), TacValue::Text(control.to_owned()))],
        );
        let values = TacScanner::new(&answer).object();
        Ok(tac_entry(&values, "granted").is_some_and(TacValue::as_flag))
    }

    pub fn minimize(&self) -> Result<(), String> {
        self.invoke("minimize", Vec::new())
    }
    pub fn maximize(&self) -> Result<(), String> {
        self.invoke("maximize", Vec::new())
    }
    pub fn fullscreen(&self) -> Result<(), String> {
        self.invoke("fullscreen", Vec::new())
    }
    pub fn close(&self) -> Result<(), String> {
        self.invoke("close", Vec::new())
    }
    pub fn set_title(&self, title: &str) -> Result<(), String> {
        self.invoke(
            "title",
            vec![(String::from("title"), TacValue::Text(title.to_owned()))],
        )
    }
    pub fn resize(&self, width: u32, height: u32) -> Result<(), String> {
        Self::valid_size(width, height).and_then(|()| {
            self.invoke(
                "resize",
                vec![
                    (String::from("width"), TacValue::Int(width.into())),
                    (String::from("height"), TacValue::Int(height.into())),
                ],
            )
        })
    }

    fn valid_size(width: u32, height: u32) -> Result<(), String> {
        ((64..=16384).contains(&width) && (64..=16384).contains(&height))
            .then_some(())
            .ok_or_else(|| String::from("invalid"))
    }

    fn invoke(&self, operation: &str, values: Vec<(String, TacValue)>) -> Result<(), String> {
        self.granted(operation)?
            .then_some(())
            .ok_or_else(|| String::from("denied"))
            .and_then(|()| self.execute(operation, values))
    }

    fn execute(&self, operation: &str, values: Vec<(String, TacValue)>) -> Result<(), String> {
        let answer = self.scope.call(&format!("window.{operation}"), values);
        let result = TacScanner::new(&answer).object();
        tac_entry(&result, "ok")
            .is_some_and(TacValue::as_flag)
            .then_some(())
            .ok_or_else(|| String::from("unavailable"))
    }
}
