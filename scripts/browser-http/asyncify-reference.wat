(module
  (import "tachyon" "http" (func $http (result i32)))
  (import "tachyon" "use" (func $use (param externref) (result i32)))
  (memory 1)
  (func (export "request") (param externref) (result i32)
    (local i32)
    (local.set 1 (call $http))
    (i32.add (local.get 1) (call $use (local.get 0)))))
