# Play Store listing assets

Store copy is **user-facing text**, so it lives in the repo and follows
`skills/copywriting.md` like every other string — not retyped into a web form
from a chat log. Play Console is where it is *pasted*, not where it is authored.

| File | Play Console field | Limit |
|---|---|---|
| `short-description.txt` | Short description | 80 chars |
| `full-description.txt` | Full description | 4000 chars |
| `feature-graphic.png` | Feature graphic | 1024×500, ≤15 MB |
| `make-feature-graphic.py` | — (regenerates the above) | — |

**App icon** is not duplicated here: Play wants 512×512 and
`app/src-tauri/icons/icon.png` is already exactly that. Uploading the same file
the app ships with is the point — a separately maintained store icon is how the
two drift apart.

**Screenshots are missing on purpose.** Play requires 2–8 phone screenshots at
16:9 or 9:16. The images in `app/screenshots/` are 720×820 desktop captures of
the pre-July UI — the wrong shape *and* the wrong app. They must be taken on a
real Android device.

## Why the copy says what it says

- It leads with what Verdure *is*, not with "AI-powered". The category is
  crowded; the specific claim ("runs on your phone") is the differentiator.
- **It states the limits.** A model small enough for a phone is not as capable
  as a data-centre one, and the description says so, including that it can be
  wrong about dates, numbers and names. `knowledge/06` makes honest expectations
  a product value, and a store listing is the first place a user tests whether
  we mean it. Overselling here is paid back in one-star reviews.
- No superlatives, no urgency, no invented social proof — `skills/copywriting.md`
  prohibits all three.

## Regenerating the feature graphic

    python3 app/store/play/make-feature-graphic.py

Palette and type follow `skills/design.md` ("Visual identity" — midnight glass).
Keep content centred: Play crops and overlays the edges in some placements.
