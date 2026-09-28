#[Service]
#[derive(Default)]
struct TicketService {
    clock: Clock,
    secure_random: SecureRandom,
    ticket: String,
    issued_at: f64,
}

impl TicketService {
    fn issue(&mut self) {
        self.ticket = self.secure_random.uuid().unwrap_or_default();
        self.issued_at = self.clock.now_ms();
    }
}
