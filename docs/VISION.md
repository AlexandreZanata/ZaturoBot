# Vision

The owner defines goals; ZaturoBot plans, executes, verifies, records, and reports work.
The system serves AlexandreZanata's authorized devices, projects, and accounts. It is
not initially a multi-tenant service. Owner control and explicit permissions outrank
autonomy, including during unattended overnight work.

A 1 GB Oracle Linux VPS is the control plane. An older Linux notebook with 22 GB RAM
is the execution plane. Browser/mobile/desktop interfaces form the client plane.
A client may disconnect while work continues. The notebook connects outbound so the
home router needs no forwarded port. Both processes restart independently.

The foundation proves transport, persistence, bounded execution, and recovery without
an AI API key. Later phases can add coding tasks, research, authorized video jobs,
price checks, Gmail drafts, and desktop observation on top of those contracts.

Success means useful work can survive interruptions and remain understandable to its
owner and future coding agents. Operational simplicity and reliable boundaries matter
more than feature count. See [requirements](REQUIREMENTS.md) and [roadmap](ROADMAP.md).
