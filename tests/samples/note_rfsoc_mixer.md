
# RFSoC Mixer Design Notes

Overview of the RF System-on-Chip mixer block used in the high-frequency optical
front-end. This document covers signal chain topology, filter specifications, and
simulation harness configuration.

---

## Signal Chain Topology

The full signal path from antenna input to optical modulator output consists of
three sequential processing stages:

```simby
[Antenna Input] ──► [LNA Block] ──► [RFSoC Mixer] ──► [IF Filter] ──► [Optical Modulator]
```

Key design constraints:

- **Noise figure:** < 3 dB across the full 2–18 GHz sweep band
- **Conversion gain:** +12 dB nominal at 10 GHz centre frequency
- **IP3:** > +25 dBm to handle adjacent channel interferers
- **LO leakage:** < −60 dBc at the RF port

---

## Filter Block Specifications

The IF filter following the mixer is a 7th-order Chebyshev bandpass design.

| Parameter         | Min     | Nominal  | Max     | Unit |
|-------------------|---------|----------|---------|------|
| Centre frequency  | 990     | 1000     | 1010    | MHz  |
| 3 dB bandwidth    | 48      | 50       | 52      | MHz  |
| Passband ripple   | —       | 0.1      | 0.5     | dB   |
| Stopband rejection| 45      | —        | —       | dB   |
| Group delay var.  | —       | —        | 2.5     | ns   |

---

## Workspace Layout

Interactive workspace canvas for the signal chain inspector tool:

```a2ui
{
  "view": "signal-chain",
  "blocks": [
    { "id": "lna",      "label": "LNA",             "gain_db": 14 },
    { "id": "mixer",    "label": "RFSoC Mixer",      "lo_freq_ghz": 9 },
    { "id": "if_filt",  "label": "IF Filter",        "bw_mhz": 50 },
    { "id": "opt_mod",  "label": "Optical Modulator","vpi_v": 3.5 }
  ],
  "connections": ["lna->mixer", "mixer->if_filt", "if_filt->opt_mod"]
}
```

---

## HDL Simulation Snippet

Reference testbench excerpt for the mixer block functional verification:

```vhdl
-- RFSoC Mixer Testbench (excerpt)
signal rf_in  : std_logic_vector(11 downto 0);
signal lo_clk : std_logic := '0';
signal if_out : std_logic_vector(11 downto 0);

lo_clk <= not lo_clk after 55.5 ps;  -- 9 GHz LO

uut: entity work.rfsoc_mixer
    port map (
        rf_in  => rf_in,
        lo_clk => lo_clk,
        if_out => if_out
    );
```

---

## Known Issues

1. **LO feed-through at cold start** — the mixer exhibits a transient LO leakage
   spike of ~−40 dBc for the first 500 µs after power-on. Mitigation: hold RF
   path in blanking state during the warm-up window.
2. ~~Spurious at 3× LO harmonic~~ — resolved in revision 2 via balanced topology.
3. Group delay variation exceeds spec at band edges when ambient temperature
   drops below −10 °C. See [thermal compensation note](./note_thermal_comp.md).

---

## References

- [ADI RFSoC Product Brief](https://www.analog.com/rfsoc)
- Pozar, D.M. *Microwave Engineering*, 4th ed., Wiley, 2012.
- Internal design review slides: `doc/reviews/rfsoc_mixer_rev3_review.pdf`
