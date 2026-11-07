use super::host::write;
use crate::Failure;
use std::path::Path;

pub(super) struct AndroidShrinking;

impl AndroidShrinking {
    pub(super) const RULES_FILE: &'static str = "proguard-rules.pro";
    pub(super) const RELEASE_BUILD_TYPE: &'static str = r#"    getByName("release") {
      isMinifyEnabled = true
      isShrinkResources = true
      proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
"#;
    pub(super) const KEEP_RULES: &'static str = "\
-keepattributes Signature,InnerClasses,EnclosingMethod,*Annotation*,MethodParameters
-keep class !androidx.**,!kotlin.**,!kotlinx.**,!org.jetbrains.**,!org.intellij.**,!com.google.**,** { *; }
-keepclasseswithmembernames,includedescriptorclasses class * {
  native <methods>;
}
-keepclassmembers class * {
  @android.webkit.JavascriptInterface <methods>;
}
";

    pub(super) fn write_rules(project: &Path) -> Result<(), Failure> {
        write(
            &project.join("app").join(Self::RULES_FILE),
            Self::KEEP_RULES.as_bytes(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AndroidShrinking;

    #[test]
    fn release_shrinking_keeps_every_class_the_app_and_its_services_declare() {
        let rules = AndroidShrinking::KEEP_RULES;
        assert!(rules.contains("-keep class !androidx.**,!kotlin.**,"));
        assert!(rules.contains(",** { *; }\n"));
        assert!(rules.contains("native <methods>;"));
        assert!(rules.contains("@android.webkit.JavascriptInterface <methods>;"));
        assert!(rules.contains("Signature"));
        let release = AndroidShrinking::RELEASE_BUILD_TYPE;
        assert!(release.contains("isMinifyEnabled = true"));
        assert!(release.contains("isShrinkResources = true"));
        assert!(release.contains(AndroidShrinking::RULES_FILE));
    }
}
