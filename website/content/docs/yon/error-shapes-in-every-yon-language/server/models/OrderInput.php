<?php

#[Checked]
class OrderInput {
    #[NotEmpty]
    #[Length({"min":1,"max":80})]
    public string $name = '';
    #[Required]
    #[Min(1)]
    #[Max(9)]
    public int $level = 0;
}
