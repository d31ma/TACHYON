type PureValidator struct{}

func (PureValidator) Check(
	// @Check(ProductInput)
	input BehaviorValue,
) (BehaviorValue, error) { return input, nil }
