macro_rules! one_test_per_case {
    ($module:ident, $check:ident, all $all:expr, [$($name:ident => $case:expr),+ $(,)?]) => {
        mod $module {
            #[allow(unused_imports)]
            use super::*;

            $(
                #[test]
                fn $name() {
                    $check($case);
                }
            )+

            #[test]
            fn every_case_has_its_own_test() {
                let mut tested = vec![$($case),+];
                let mut all: Vec<_> = $all.into_iter().collect();
                tested.sort();
                all.sort();
                assert_eq!(tested, all, "every case needs its own test");
            }
        }
    };
    ($module:ident, async $check:ident, all $all:expr, [$($name:ident => $case:expr),+ $(,)?]) => {
        mod $module {
            #[allow(unused_imports)]
            use super::*;

            $(
                #[tokio::test]
                async fn $name() {
                    $check($case).await;
                }
            )+

            #[test]
            fn every_case_has_its_own_test() {
                let mut tested = vec![$($case),+];
                let mut all: Vec<_> = $all.into_iter().collect();
                tested.sort();
                all.sort();
                assert_eq!(tested, all, "every case needs its own test");
            }
        }
    };
}

pub(crate) use one_test_per_case;
