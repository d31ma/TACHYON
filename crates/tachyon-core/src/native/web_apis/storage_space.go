package tac

type StorageSpace struct{}

func (StorageSpace) Estimate() error {
	return Web.Call("storageSpace", "estimate", nil, nil)
}

func (StorageSpace) Persist() error {
	return Web.Call("storageSpace", "persist", nil, nil)
}
