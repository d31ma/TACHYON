<?php
#[Checked]
class ProductInput {
    #[Required]
    #[NotEmpty]
    #[Length({"min":2,"max":4})]
    public string $name='';
    #[Min(0)]
    #[Max(100)]
    public int $price=0;
}

