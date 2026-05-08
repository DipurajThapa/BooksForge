# BooksForge — End User License Agreement (Draft)

**Effective date:** *(to be set on first public release)*
**Status:** **Draft — pending legal review.** Replace this notice
with an effective date before any public download is offered.

> *Refs:* `LICENSE` (provisional placeholder),
> `PRIVACY_POLICY.md`, `TERMS_OF_SERVICE.md`,
> `EXTERNAL_AUDIT_BACKLOG.md #46`.

---

## 1. Parties and acceptance

This End User License Agreement ("**EULA**") is a legal agreement
between you (either an individual or a single legal entity, the
"**User**") and BooksForge contributors (the "**Licensor**")
regarding the BooksForge desktop software, including all associated
documentation, fonts, sample files, sidecar binaries (Pandoc,
EPUBCheck), and updates (collectively, the "**Software**").

By installing, copying, or otherwise using the Software, you accept
the terms of this EULA. If you do not accept the terms, do not
install or use the Software.

This EULA applies **in addition to** the per-component licenses
listed in `THIRD_PARTY_LICENSES.md`. Where a term in this EULA
conflicts with a permissive open-source license that governs a
specific component, the open-source license governs that component
to the extent of the conflict.

---

## 2. License grant

Subject to your compliance with this EULA, the Licensor grants you a
**non-exclusive, non-transferable, royalty-free** license to:

- install and use one (1) copy of the Software on each computer that
  you own or control, for personal or commercial creative-writing
  purposes;
- make a reasonable number of backup copies for archival purposes;
- create, modify, export, and distribute the **content** you
  produce with the Software (your manuscripts, outlines, exports —
  collectively, your "**User Content**") without any further
  permission, royalty, or attribution to the Licensor.

This license is **per-user, per-machine** in the MVP. Multi-machine
or multi-user licensing models may be offered in future versions
(see `outputs/MVP_SCOPE.md §3`).

---

## 3. Ownership of your content

**You own everything you write.** The Licensor does not claim any
ownership, copyright, or license over your User Content. The Software
runs entirely on your computer; the Licensor does not see, store, or
process your manuscripts. Refer to `PRIVACY_POLICY.md` for the
complete privacy posture.

The fact that AI features (Outline Architect, Copyedit, Continuity,
Humanization, etc.) helped produce a passage does **not** transfer
any ownership to the Licensor. Whether AI-assisted output is
copyrightable in your jurisdiction is a separate legal question
between you and the laws that apply to you; this EULA does not
modify that question in either direction.

---

## 4. Restrictions

You may not:

- reverse engineer, decompile, or disassemble the Software, except to
  the extent that such activity is expressly permitted by applicable
  law notwithstanding this restriction;
- remove or alter copyright, trademark, or other proprietary notices
  on or in the Software;
- redistribute the Software's installer, signed binaries, or
  sidecars (Pandoc, EPUBCheck) as part of another product or service
  without the Licensor's written permission;
- use the Software's name, logos, or trademarks in a manner that
  implies endorsement of your product or service by the Licensor.

You **may**:

- write, publish, sell, and self-publish any work you create with
  the Software, in any genre, on any platform, in any quantity,
  without limitation;
- run the Software on as many of your own machines as you like;
- describe your work as "written with BooksForge" in marketing
  copy, provided this does not imply endorsement of you or your
  work by the Licensor.

---

## 5. Updates and auto-updates

The Software may include an opt-out auto-updater. By default, on
launch, the Software checks an update endpoint controlled by the
Licensor and may download a newer version. Auto-updates can be
disabled at any time in *Settings → Updates*.

The Licensor may release updates that:

- fix defects;
- improve performance;
- add or change features;
- update bundled sidecar binaries;
- adjust the model registry that the Setup Wizard offers.

The Licensor is **not obligated** to provide updates. Continuing to
use a version of the Software past its effective end of support is
permitted but unsupported.

---

## 6. Sidecar binaries (Pandoc, EPUBCheck) and bundled fonts

The Software bundles or invokes third-party components, each governed
by its own license:

- **Pandoc** runs as a separate process invoked by the Software for
  DOCX / PDF export. Pandoc is licensed under GPLv2-or-later. Because
  the Software invokes Pandoc as an external process (not statically
  linked), this EULA does not place GPL conditions on the Software
  itself, but the Pandoc binary remains under its own license.
- **EPUBCheck** runs as a separate process for EPUB validation,
  under Apache-2.0 + W3C terms.
- **Fonts** bundled with the Software are listed in
  `THIRD_PARTY_LICENSES.md` with their respective licenses (typically
  SIL Open Font License 1.1 or Apache-2.0).

You may use these components as part of the Software's normal
operation. Distributing them outside that context is governed by
their own licenses, not this EULA.

---

## 7. Disclaimers

THE SOFTWARE IS PROVIDED **"AS IS"** WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, AND NON-
INFRINGEMENT. IN NO EVENT SHALL THE LICENSOR BE LIABLE FOR ANY
CLAIM, DAMAGES, OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT, OR OTHERWISE, ARISING FROM, OUT OF, OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

In particular, the Licensor makes no warranty that the AI-assisted
features will produce accurate, original, non-infringing, or
publishable output. Reviewing the Software's suggestions before
accepting them is your responsibility.

The Licensor is **not** a publisher and does not provide editorial,
legal, or publishing advice.

---

## 8. Limitation of liability

To the maximum extent permitted by applicable law, the Licensor's
total cumulative liability arising out of or related to this EULA or
the Software shall not exceed the greater of:

(a) the amount you paid for the Software in the twelve months
    preceding the event giving rise to the liability, or

(b) ten US dollars ($10).

The Licensor shall not be liable for indirect, incidental, special,
consequential, exemplary, or punitive damages, including without
limitation lost profits or lost data, even if advised of the
possibility of such damages.

---

## 9. Termination

This EULA terminates automatically if you fail to comply with its
terms. On termination, you must stop using the Software and remove
all copies. Sections 3 (Ownership of your content), 7
(Disclaimers), 8 (Limitation of liability), and 10 (Governing law)
survive termination.

---

## 10. Governing law

This EULA is governed by the laws of *(jurisdiction to be set on
first public release — recommend the founder's jurisdiction)*,
without regard to its conflict-of-laws principles. Any dispute
arising under or related to this EULA shall be resolved in the
courts of that jurisdiction.

If you are a consumer in a jurisdiction whose mandatory consumer-
protection laws override the choice of law above, those laws apply
to you instead.

---

## 11. Severability

If any provision of this EULA is held unenforceable, the remaining
provisions shall continue in full force and effect.

---

## 12. Contact

- Licensing questions: *(licensing@booksforge.app — pending)*
- Legal notices: *(legal@booksforge.app — pending)*

---

*This file is a draft. Before any public download is offered, it
must be reviewed by legal counsel, jurisdiction-specific consumer
clauses must be added or removed as appropriate, contact addresses
must be provisioned, and the licensing model (free / paid / hybrid)
documented in `docs/BUSINESS_MODEL.md` must be reconciled with
section 2 of this EULA.*
