"""Task bank for usaf_memo@0.3.0 authoring evals."""

from __future__ import annotations

from typing import Any

Task = dict[str, Any]


def _t(
    tid: str,
    category: str,
    prompt: str,
    *,
    difficulty: str = "medium",
    expect_indorsement: bool | None = None,
    traps: list[str] | None = None,
) -> Task:
    return {
        "id": tid,
        "category": category,
        "difficulty": difficulty,
        "expect_indorsement": expect_indorsement,
        "traps": traps or [],
        "prompt": prompt,
    }


TASKS: list[Task] = [
    _t(
        "mfr-01",
        "mfr",
        "Write a Memorandum for Record dated 2026-03-12. 88th Communications Squadron, Wright-Patterson AFB. "
        "Capt Jordan Hale, Flight Commander, records that the squadron completed a no-notice COOP tabletop and "
        "identified three action items for the next UCC meeting. No MEMO FOR addressee other than the record; "
        "this is an MFR so FROM should be blank. No indorsement. Subject: Continuity of Operations Tabletop After Action.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["blank_from", "drop_blueprint_indorsement"],
    ),
    _t(
        "mfr-02",
        "mfr",
        "Memorandum for Record from the 1st Fighter Wing safety office. Maj Avery Chen documents a near-miss "
        "on the flightline involving a AGE tug on 2 April 2026. Include a short roster of witnesses in a block quote "
        "(names with ranks). No FROM line. No indorsements. Signature: AVERY CHEN, Maj, USAF / Chief of Safety.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["block_quote_roster", "drop_blueprint_indorsement"],
    ),
    _t(
        "simple-01",
        "simple_tasking",
        "Write a USAF official memo. MEMO FOR: 88 ABW/CC. FROM: 88 CS/CC, 88th Communications Squadron, "
        "2010 Monahan Way, Wright-Patterson AFB OH 45433-5302. Subject: Tasking for Cyber Defense Exercise Support. "
        "Date 2026-05-01. Col PATRICE N. OKONKWO, Commander, directs the wing to provide two network defenders "
        "to the 26 NOS for CYBER FLAG 26. Two numbered paragraphs: purpose, then required action with a suspense "
        "of 15 May 2026. Nested bullets for the two billets (1N4X1A and 17S). No attachments, no indorsement, unclassified.",
        difficulty="easy",
        expect_indorsement=False,
    ),
    _t(
        "simple-02",
        "simple_tasking",
        "USAF memo from 4 FW/CC to 4 OSS/CC. Subject: Additional Manning for Night Flying Operations. "
        "The commander wants the OSS to identify five extra SOF-qualified controllers for a 30-day surge starting "
        "1 June 2026. Signature: MARCUS D. ELLISON, Col, USAF, Commander. Date 2026-05-20. Keep it to three short "
        "paragraphs. Do not add an indorsement card.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["colon_in_subject"],
    ),
    _t(
        "simple-03",
        "simple_tasking",
        "Write a memo FOR 56 FW/CC FROM 56 MXG/CC. Subject: Aircraft Availability Briefing (Suspense: 8 Jul 2026). "
        "Date 2026-06-24. Lt Col SAMIRA Q. PATEL, Deputy Commander, signs FOR THE COMMANDER. Two paragraphs plus "
        "a bullet list of three jets down for parts. No classification banner. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["authority_line", "suspense_in_subject"],
    ),
    _t(
        "yaml-colon-01",
        "yaml_traps",
        "Official memo. FOR: HQ ACC/A3. FROM: 1 FW/CC. Subject: Request: Additional Manning for QRA. "
        "The subject MUST contain a colon after Request. Date 2026-02-18. Col JAMES R. WHITAKER, Commander. "
        "One paragraph asking ACC to plus-up two 11F billets. Unclassified. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["colon_in_subject"],
    ),
    _t(
        "yaml-italics-01",
        "yaml_traps",
        "Memo FOR: 88 ABW/JA FROM: 88 ABW/CC. Subject must italicize the publication: Implementation of *AFH 33-337* "
        "Tongue and Quill Standards. Date 2026-01-09. Col LENA M. BROOKS, Commander. Body cites the handbook and "
        "directs a staff rewrite of local prep letters. Include one reference: AFH 33-337, 27 July 2023, *The Tongue and Quill*. "
        "No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["italics_in_subject", "italics_in_references"],
    ),
    _t(
        "yaml-date-01",
        "yaml_traps",
        "Write a memo dated 15 September 2026 (store the date field in YYYY-MM-DD). FOR 42 ABW/CC FROM 42 CS/CC. "
        "Subject: Fiscal Year 27 Communications Budget Lock. Signature: RHEA T. GOMEZ, Lt Col, USAF, Commander. "
        "Do not put '15 September 2026' in the date field; convert it. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["date_format"],
    ),
    _t(
        "yaml-fontsize-01",
        "yaml_traps",
        "DAF headquarters-style memo (memo_style daf) with body font size 11.5. FOR SAF/CN FROM AF/A6. "
        "Subject: Enterprise IT Service Catalog Refresh. Date 2026-08-03. SES signature: "
        "ALAN P. MORRIS, SES, Director of Cyberspace Operations. Two paragraphs, first-line indent style implied by DAF. "
        "No indorsement. Unclassified.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["font_size_decimal", "daf_style"],
    ),
    _t(
        "cui-01",
        "cui",
        "Write a CUI memo. FOR: 88 MDG/CC FROM: 88 MDG/SG. Subject: Privacy Review of Outpatient Records Transfer. "
        "Date 2026-04-07. Classification CUI, controlled_by 88 MDG/SGH, poc Capt Nina Volkov, DSN 787-4410, "
        "nina.volkov@us.af.mil, category Privacy/MED, limited_dissemination FEDONLY, dissemination NF. "
        "Signature: NINA VOLKOV, Capt, USAF, Health Information Manager. Two paragraphs describing a records "
        "transfer to a civilian hospital. No indorsement. Do not include SECRET; this is CUI only.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields"],
    ),
    _t(
        "cui-02",
        "cui",
        "CUI memo for 16 AF/A2 from 67 CW/IN. Subject: CTI Sharing Procedures for Coalition Partners. "
        "CUI category CTI, controlled_by 67 CW/IN, poc Maj Owen Blake, DSN 312-555-0199, owen.blake@us.af.mil, "
        "limited dissemination NOFORN. Date 2026-07-11. Signature OWEN BLAKE, Maj, USAF, Director of Intelligence. "
        "Include two references with italicized titles. No attachments. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields", "italics_in_references"],
    ),
    _t(
        "cui-03",
        "cui",
        "Prepare a CUI official memorandum announcing a PII incident response. MEMO FOR SEE DISTRIBUTION. "
        "FROM: 502 ABW/CC, Joint Base San Antonio, 1 Washington Circle, JBSA-Fort Sam Houston TX 78234. "
        "Distribution: 502 ABW/JA, 502 ABW/A1, 502 CS/CC, 502 CONS/CC. Subject: Personally Identifiable Information Incident (Suspense: 22 May 2026). "
        "CUI, controlled_by 502 ABW/A6, poc SMSgt Dana Ruiz, DSN 312-429-7701, dana.ruiz@us.af.mil, category PRVCY. "
        "Col HARPER L. NGUYEN, Commander. Three paragraphs plus bullets for reporting steps. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["see_distribution", "cui_variant_fields", "colon_in_subject"],
    ),
    _t(
        "class-secret-01",
        "classified",
        "SECRET memo (classification SECRET, no CUI fields). FOR: 25 AF/CC FROM: 9 RW/CC. "
        "Subject: Reconnaissance Orbit Realignment. Date 2026-03-30. Col DIEGO S. FARROW, Commander. "
        "Dissemination NF. Two short paragraphs of unclassified-looking placeholder operational language "
        "(this is a format exercise, not real classified content). No indorsement. Do not mark it CUI.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["classification_enum", "no_cui_fields_on_secret"],
    ),
    _t(
        "class-ts-01",
        "classified",
        "TOP SECRET banner memo for 8 AF/CC from 509 BW/CC. Subject: Commander's Nuclear Surety Observation. "
        "Date 2026-09-01. Brig Gen ALICIA M. GRANT, Commander. Unclassified body text that only exercises the "
        "banner fields. No CUI variant fields. No indorsement. letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS EIGHTH AIR FORCE.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["classification_enum"],
    ),
    _t(
        "class-unclass-banner-01",
        "classified",
        "Write an unclassified memo that STILL prints the UNCLASSIFIED banner (classification UNCLASSIFIED, not blank). "
        "FOR 355 FW/CC FROM 355 OG/CC. Subject: Open House Parking Plan. Date 2026-08-15. "
        "Col TOMAS E. RIVERA, Commander. Two paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["unclassified_explicit_vs_blank"],
    ),
    _t(
        "ind-01",
        "indorsement",
        "Tasking memo from 1 SOW/CC to 1 SOG/CC, Subject: Deployment Risk Assessment, date 2026-02-02, "
        "Col FIONA K. ADEYEMI, Commander, two paragraphs. Then add a 1st Ind from 1 SOG/CC back to 1 SOW/CC "
        "dated 2026-02-05, Col BRETT L. HADDAD, Commander, action approve, format standard, body: "
        "I approve the deployment risk assessment and will provide the requested OPREP within 48 hours.",
        difficulty="medium",
        expect_indorsement=True,
        traps=["indorsement_card"],
    ),
    _t(
        "ind-02",
        "indorsement",
        "Memo FOR 18 WG/CC FROM 18 OG/CC, Subject: F-22 Night Flying Waiver, date 2026-01-14, "
        "Col INGRID P. CHO, Commander. Then two indorsements: 1st Ind 18 WG/CC to PACAF/A3 dated 2026-01-16, "
        "action concur (approve), format standard, Brig Gen LEON W. PRUITT; 2d Ind PACAF/A3 to 18 WG/CC dated "
        "2026-01-20, action approve, format separate_page, Maj Gen KAREN D. ISHIKAWA. Each indorsement needs its "
        "own signature block and a one-sentence body.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["two_indorsements", "separate_page", "approve_chain"],
    ),
    _t(
        "ind-03",
        "indorsement",
        "Write a short staffing memo FOR 88 ABW/JA FROM 88 CS/CC, Subject: Legal Review of Network Monitoring Charter, "
        "date 2026-06-01, Lt Col HUGO R. SANTOS. Add an informal indorsement (format informal) from 88 ABW/JA "
        "to 88 CS/CC with action undecided, empty-ish body is not allowed — give a one-line 'staffing for signature' "
        "body so the informal signature block has text to ride on. Date on the indorsement left blank for the signer.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["informal_indorsement", "undecided_action", "blank_indorsement_date"],
    ),
    _t(
        "ind-04",
        "indorsement",
        "Policy memo from HQ AFMC/A4 to 76 MXW/CC, Subject: Depot Throughput Reporting Change, date 2026-04-22, "
        "FOR THE COMMANDER signed by SES MARIA E. CALDWELL, Acting Director of Logistics. Add a 1st Ind from "
        "76 MXW/CC to HQ AFMC/A4, action disapprove, format standard, Col NATHAN Y. BROOKS, Commander, body "
        "explains the depot cannot meet the new 48-hour metric without overtime funding.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["authority_line", "disapprove"],
    ),
    _t(
        "dist-01",
        "distribution",
        "MEMO FOR must be exactly SEE DISTRIBUTION. FROM: HQ ACC/A1, 205 Dodd Blvd, Joint Base Langley-Eustis VA 23665. "
        "Subject: Civilian Hiring Freeze Guidance. Date 2026-10-01. Distribution list of six offices: "
        "1 FW/CC, 4 FW/CC, 20 FW/CC, 366 FW/CC, 355 FW/CC, 388 FW/CC. Col PRANAV S. MEHTA, Director of Manpower. "
        "Two paragraphs. Courtesy copy: AF/A1. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["see_distribution", "cc"],
    ),
    _t(
        "att-01",
        "attachments",
        "Memo FOR 635 SCOW/CC FROM 635 SCOW/A3. Subject: Exercise After Action Package. Date 2026-05-18. "
        "Lt Col IVY N. COLE, Director of Operations. Body mentions two attachments in order. Attachments: "
        "1) After Action Report, 2026 May 12; 2) *ACC EXORD 26-04* Extract, 2026 May 10. Also two references "
        "with italic titles. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["attachments_order", "italics_in_attachments"],
    ),
    _t(
        "att-02",
        "attachments",
        "Single-attachment memo. FOR 9 AF/CC FROM 20 FW/CC. Subject: Mishap Interim Safety Message. Date 2026-07-07. "
        "Col OWEN F. BARR, Commander. One attachment only: Interim Safety Message, 2026 Jul 06 — unnumbered when alone. "
        "Do not write 'as stated'. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["single_attachment_unnumbered"],
    ),
    _t(
        "daf-01",
        "daf_style",
        "This must be a DAF headquarters memorandum (memo_style: daf), not USAF. FOR: ALMAJCOM FROM: HQ USAF/A3. "
        "Letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS UNITED STATES AIR FORCE. Date 2026-11-02. "
        "Subject: Ready Aircrew Program Adjustments. Gen-level signature: DAVID H. SLOANE, Lt Gen, USAF, "
        "Deputy Chief of Staff for Operations. Three unnumbered-style paragraphs with a first-line indent implied. "
        "No indorsement. Unclassified, no banner.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["daf_style", "almajcom"],
    ),
    _t(
        "joint-01",
        "letterhead",
        "Joint command letterhead: first line UNITED STATES INDO-PACIFIC COMMAND, second line HEADQUARTERS USINDOPACOM. "
        "Seal dod. Seal subtitle INDOPACOM. Tag line *Prepare / Prevail / Peace*. FOR PACAF/CC FROM J3. "
        "Subject: Air Tasking Order Coordination Window. Date 2026-03-03. RDML (use USAF format anyway) "
        "actually sign as COLIN J. PARK, Col, USAF, J3 Air. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["joint_letterhead", "dod_seal", "tag_line_italics"],
    ),
    _t(
        "ussf-01",
        "space_force",
        "Space Force memo. Letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS UNITED STATES SPACE FORCE. "
        "Seal dow. FOR SpOC/CC FROM S3. Subject: Satellite Catalog Hygiene Sprint. Date 2026-12-01. "
        "Signature: KIRA L. DONOVAN, Col, USSF, Director of Operations. Two paragraphs plus a nested list of "
        "three catalog clean-up tasks. No indorsement. Unclassified.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["ussf_service"],
    ),
    _t(
        "civ-01",
        "civilian",
        "Civilian-signed memo. FOR 88 ABW/CC FROM 88 ABW/FM. Subject: Overtime Ceiling for Fourth Quarter. "
        "Date 2026-08-20. Signature: PATRICIA A. KOHL, GS-15, Comptroller — no military grade. Two paragraphs. "
        "No indorsement. USAF style.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["civilian_signature"],
    ),
    _t(
        "body-quote-01",
        "body_format",
        "Memo FOR 437 AW/CC FROM 437 OSS/CC. Subject: Distinguished Visitor Itinerary. Date 2026-09-09. "
        "Lt Col NOAH E. SIMMS, Commander. Body: one intro paragraph, then a block quote itinerary of four lines "
        "with trailing backslashes for line breaks (time, location, escort), then a closing paragraph. "
        "No indorsement. This is about body typesetting, not attachments.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["block_quote_linebreaks"],
    ),
    _t(
        "body-nested-01",
        "body_format",
        "Write a USAF memo with rich nested lists. FOR 23 WG/CC FROM 23 OG/CC. Subject: Weapons School Nomination Package. "
        "Date 2026-01-28. Col AMELIA R. VICK, Commander. First paragraph purpose. Then bullets: three nominees, each with "
        "nested bullets for MDS, hours, and last evaluation. Do not manually number top-level paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["nested_lists", "no_manual_numbering"],
    ),
    _t(
        "long-01",
        "long_body",
        "Long policy memo, five top-level paragraphs plus two nested lists, about a new 24/7 help desk at 26 NOS. "
        "FOR 67 CW/CC FROM 26 NOS/CC. Subject: Network Operations Help Desk Concept of Operations. Date 2026-02-27. "
        "Lt Col CESAR A. MOLINA, Commander. Cover mission, manning (include a small roster block quote of four names), "
        "hours, escalation, and a suspense of 15 March 2026 for comments. CC 67 CW/A3. No indorsement. Unclassified.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["long_body", "cc", "block_quote_roster"],
    ),
    _t(
        "minimal-01",
        "minimal",
        "Smallest valid USAF memo you can write. FOR 99 ABW/CC. Subject: Gate 3 Closure. Signature FIRST LAST, Capt, USAF, "
        "and a duty title. One sentence body: Gate 3 will close for paving 3–5 October 2026. Leave optional fields at defaults. "
        "No indorsement. Do not include FROM (same installation implied — actually include FROM as 99 ABW/A7CE only, no mailing address).",
        difficulty="easy",
        expect_indorsement=False,
        traps=["drop_blueprint_indorsement", "optional_defaults"],
    ),
    _t(
        "minimal-02",
        "minimal",
        "Produce a memo that fills only obliged fields and deletes every defaulted optional field you do not need. "
        "FOR: 7 BW/CC. Subject: Fuel Hydrant Outage. Signature: TARA S. QUINN, Maj, USAF, Operations Officer. "
        "One paragraph. No date (use today). No FROM (MFR-like? No — this IS addressed, so FROM should be 7 BW/CP with no street). "
        "No classification, no attachments, no indorsement, no tag line.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["omit_optional_fields", "drop_blueprint_indorsement"],
    ),
    _t(
        "person-for-01",
        "addressing",
        "Address a specific person in MEMO FOR using the parenthetical pattern: 309 AMXG/CC (COL JANE DOE). "
        "FROM: 76 MXW/CC. Subject: Depot Artisan Overtime Approval. Date 2026-04-04. "
        "Col VICTOR M. LANG, Commander. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["memo_for_person"],
    ),
    _t(
        "multi-for-01",
        "addressing",
        "MEMO FOR three offices on the same installation: 1 FW/CC, 1 FW/CV, 1 FW/CCC. FROM: 1 FW/JA (office symbol only). "
        "Subject: Ethics Training Attendance. Date 2026-05-05. Lt Col BRIAN O. FELDER, Staff Judge Advocate. "
        "One paragraph plus bullets of three dates. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["multi_memo_for"],
    ),
    _t(
        "offbase-from-01",
        "addressing",
        "Recipient is off-base, so FROM must be a full mailing address. FOR: HQ AFSOC/A3, Hurlburt Field. "
        "FROM: 27 SOW/CC, 100 Air Commando Way, Cannon AFB NM 88103-5000 (four-line FROM). "
        "Subject: MC-130J Simulator Scheduling. Date 2026-06-06. Col NADIA P. BROOKS, Commander. Two paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["full_mailing_from"],
    ),
    _t(
        "auth-01",
        "authority_line",
        "Staff member announces policy in their own area: fill authority_line FOR THE COMMANDER. "
        "FOR ALMAJCOM/A4 FROM HQ AFMC/A4. Subject: Packaging Standards for Retrograde Cargo. Date 2026-07-21. "
        "Signer is Col ELISA K. MOON, Deputy Director of Logistics, not the commander. Three short paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["authority_line"],
    ),
    _t(
        "auth-02",
        "authority_line",
        "The commander herself signs, so authority_line must be blank/omitted. FOR 355 FW/CC FROM 355 FW/CC. "
        "Wait — that is a self-memo; instead FOR 355 OG/CC FROM 355 FW/CC. Subject: Stand-Down for ORM Reset. "
        "Date 2026-08-08. Col JOANNA P. REED, Commander. Do not put FOR THE COMMANDER. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["blank_authority_when_commander"],
    ),
    _t(
        "tag-01",
        "letterhead",
        "Include an organizational motto in the footer with mixed emphasis: **Aim High** *Fly-Fight-Win*. "
        "Seal dow, subtitle UNITED STATES AIR FORCE. Letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS TWELFTH AIR FORCE. "
        "FOR 12 AF/A3 FROM 12 AF/CC. Subject: Checkered Flag 26 Planning Guidance. Date 2026-01-15. "
        "Maj Gen LUIS A. ORTEGA, Commander. Two paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["tag_line_markdown", "seal_subtitle"],
    ),
    _t(
        "seal-none-01",
        "letterhead",
        "No seal at all. Letterhead still DEPARTMENT OF THE AIR FORCE / 319th Reconnaissance Wing. "
        "FOR 319 RW/CC FROM 319 OG/CC. Subject: Alert Facility Quiet Hours. Date 2026-02-14. "
        "Col MICAH D. STONE, Commander. One paragraph. No indorsement. Leave seal empty if the schema allows, "
        "otherwise pick the option that prints no seal.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["empty_seal_enum"],
    ),
    _t(
        "cc-01",
        "cc",
        "Memo FOR 62 AW/CC FROM 62 AW/A3. Subject: Channel Mission Diversion. Date 2026-03-19. "
        "Col HELEN S. PARK, Director of Operations. CC three people: Col A. Smith, AMC/A3; Mr. B. Lee, 62 AW/FM; "
        "CMSgt C. Ortiz, 62 AW/CCC. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["cc"],
    ),
    _t(
        "refs-single-01",
        "references",
        "Exactly one reference, which should print after the subject rather than as a lettered block: "
        "AFI 90-201, 12 April 2022, *The Inspection System*. FOR 4 AF/CC FROM 4 AF/IG. "
        "Subject: UEI Observation Sharing. Date 2026-04-11. Col GINA T. MARSH, Inspector General. "
        "Two paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["single_reference", "italics_in_references"],
    ),
    _t(
        "refs-multi-01",
        "references",
        "Three references with italicized titles, including one with a colon in the title that must be quoted in YAML. "
        "Titles: AFMAN 33-326, 25 November 2011, *Preparing Official Communications*; "
        "AFI 33-360, *Publications and Forms Management*; "
        "DAFI 16-1404, *Information Security: Program Management*. "
        "FOR SAF/AA FROM 88 ABW/CC. Subject: Records Disposition Sweep. Date 2026-05-30. "
        "Col IVAN G. WELLS, Commander. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["colon_in_reference", "italics_in_references"],
    ),
    _t(
        "wrong-quill-trap-01",
        "instruction_following",
        "Write a USAF memo about snow removal at Minot, FOR 5 BW/CC FROM 5 CES/CC, date 2026-01-05, "
        "Maj COLIN P. HAYES, Commander. The $quill line must be usaf_memo@0.3.0 — do not invent another template, "
        "do not emit YAML frontmatter with ---, do not use ~~~card-yaml unless you must. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["quill_ref"],
    ),
    _t(
        "mustfill-trap-01",
        "instruction_following",
        "The blueprint contains an example indorsement card with !must_fill fields. Your memo must NOT include any "
        "indorsement. FOR 49 WG/CC FROM 49 MXG/CC. Subject: ISO Maintenance Pause. Date 2026-06-18. "
        "Col RUTH A. KLEIN, Commander. Two paragraphs. Delete the example indorsement rather than leaving placeholders.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["drop_blueprint_indorsement"],
    ),
    _t(
        "enum-bad-tempt-01",
        "enums",
        "Classification must be CONFIDENTIAL (not 'Confidential', not CUI). FOR 55 WG/CC FROM 55 OG/CC. "
        "Subject: Airborne Recording Handling. Date 2026-07-19. Col PETER J. NAIR, Commander. "
        "Dissemination blank. Two paragraphs of dummy handling guidance. No indorsement. No CUI controlled_by fields.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["classification_enum_case"],
    ),
    _t(
        "memo-style-usaf-01",
        "enums",
        "Explicitly set memo_style to usaf even though that is the default. FOR 354 FW/CC FROM 354 OG/CC. "
        "Subject: Red Flag-Alaska Beddown. Date 2026-08-28. Col YUSUF I. HADDAD, Commander. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["memo_style_usaf"],
    ),
    _t(
        "font-12-01",
        "numbers",
        "font_size must be the number 12, not the string '12'. FOR 436 AW/CC FROM 436 MXS/CC. "
        "Subject: C-5 Isochronal Inspection Delay. Date 2026-09-12. Lt Col KELLY M. BURNS, Commander. "
        "Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["unquoted_number"],
    ),
    _t(
        "font-10-01",
        "numbers",
        "Use font_size 10 (integer, unquoted) because this is a cramped info memo. FOR 92 ARW/CC FROM 92 LRS/CC. "
        "Subject: Lodging Overflow During Surge. Date 2026-10-10. Maj IVANA P. DROZ, Commander. One paragraph. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["unquoted_number"],
    ),
    _t(
        "blank-date-01",
        "dates",
        "Leave the date field blank so the renderer uses today. FOR 628 ABW/CC FROM 628 SFS/CC. "
        "Subject: Installation Access Credential Reset. Signature: WALTER J. POPE, Lt Col, USAF, Commander. "
        "Two paragraphs. No indorsement. Do not write today's date yourself.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["blank_date_means_today"],
    ),
    _t(
        "ind-action-concur-01",
        "indorsement",
        "Staffing chain of three indorsements on a waiver request. Base memo: FOR 1 FW/CC FROM 1 OG/CC, "
        "Subject: Low-Level Route Waiver, date 2026-02-08, Col ANNA S. FREY. "
        "1st Ind: 1 FW/SE to 1 FW/CC, action approve (coordinating), standard, Lt Col MARK P. YOUNG. "
        "2d Ind: 1 FW/JA to 1 FW/CC, action approve, standard, Col DEBRA L. SWAN. "
        "3d Ind: 1 FW/CC to 1 OG/CC, action approve (approval authority), separate_page, Col THEODORE M. BLAKE. "
        "Each indorsement one sentence.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["three_indorsements"],
    ),
    _t(
        "cui-minimal-01",
        "cui",
        "CUI with only the required variant fields (controlled_by and poc), category and limited_dissemination left blank. "
        "FOR 11 WG/CC FROM 11 CS/CC. Subject: Directory Listing of Unlisted DSN Numbers. Date 2026-11-11. "
        "controlled_by 11 CS/SCX, poc 1st Lt Asha Raman, DSN 312-612-4400, asha.raman@us.af.mil. "
        "Capt? Signature: ASHA RAMAN, 1st Lt, USAF, Cyber Operations Officer. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_required_only"],
    ),
    _t(
        "body-code-01",
        "body_format",
        "Body must include a fenced code block with sample log lines (use backticks, not tildes) showing three syslog lines. "
        "FOR 688 CW/CC FROM 26 NOS/CC. Subject: Sample Event Log for Ticket 26-441. Date 2026-03-22. "
        "Lt Col REESE A. KIM, Commander. Intro paragraph, the code block, closing paragraph. No indorsement. "
        "Do not use column-zero ~~~ inside the body.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["fenced_code_not_tildes"],
    ),
    _t(
        "award-01",
        "narrative",
        "Write a nomination transmittal memo. FOR 12 AF/CC FROM 355 FW/CC. Subject: Nomination of Capt Luis Mendez for the "
        "Lance P. Sijan Award. Date 2026-01-31. Col JOANNA P. REED, Commander. Three paragraphs of dense but original "
        "achievement prose (not bullet bingo). Attachment: AF Form 1206, 2026 Jan 30. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["single_attachment"],
    ),
    _t(
        "policy-01",
        "narrative",
        "HQ policy memo. FOR ALMAJCOM FROM HQ USAF/A1. Subject: Parental Leave Implementation Guidance. Date 2026-04-15. "
        "Lt Gen HELENE W. CARTER, Deputy Chief of Staff for Manpower. Authority line blank (she signs as DCS). "
        "Four paragraphs: purpose, eligibility, documentation, points of contact. CC SAF/MR. memo_style daf. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["daf_style", "cc"],
    ),
    _t(
        "ops-01",
        "narrative",
        "OPS tasker. FOR 23 WG/CC FROM ACC/A3. Subject: Immediate Force Generation for Hurricane Relief (Suspense: 4 Sep 2026). "
        "Date 2026-09-02. Maj Gen COLTON B. GRAVES, Director of Operations, FOR THE COMMANDER. "
        "Paragraph 1 situation, paragraph 2 mission, paragraph 3 specified tasks as nested bullets (airlift, medevac, comms). "
        "No classification. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["authority_line", "suspense_in_subject", "colon_in_subject"],
    ),
    _t(
        "safety-01",
        "narrative",
        "Safety memo. FOR 4 FW/CC FROM 4 FW/SE. Subject: Bird Aircraft Strike Hazard After Action. Date 2026-05-09. "
        "Lt Col PRISCILLA N. ORTIZ, Chief of Safety. Include a block-quote list of three strike times and locations "
        "with backslash line breaks. Recommend two mitigations as nested bullets. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["block_quote_linebreaks"],
    ),
    _t(
        "medical-cui-01",
        "cui",
        "Medical CUI. FOR 59 MDW/CC FROM 59 MDW/SGH. Subject: Sentinel Event Notification. Date 2026-06-30. "
        "Col ANDREW P. IBE, Chief of Medical Staff. CUI, category Privacy/MED, controlled_by 59 MDW/SGH, "
        "poc Maj Lila Cho, DSN 312-554-2090, lila.cho@us.af.mil, limited_dissemination FEDONLY. "
        "Do not invent patient names; refer to 'the member'. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields"],
    ),
    _t(
        "legal-01",
        "narrative",
        "Legal review memo. FOR 1 FW/CC FROM 1 FW/JA. Subject: Review of Proposed Off-Limits Order. Date 2026-07-13. "
        "Col DEBRA L. SWAN, Staff Judge Advocate. Two paragraphs plus a nested list of three legal bases. "
        "Reference: AFI 31-115, *Law Enforcement Operations*. CC 1 FW/SE. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["italics_in_references", "cc"],
    ),
    _t(
        "logistics-01",
        "narrative",
        "Logistics memo with a four-line FROM address. FOR HQ AMC/A4 FROM 60 AMW/LRS, 350 Hangar Ave, Travis AFB CA 94535-2802. "
        "Subject: Pallet Shortage for Mobility Exercise. Date 2026-08-01. Maj TOMAS R. GLEASON, Commander. "
        "Ask for 200 463L pallets. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["full_mailing_from"],
    ),
    _t(
        "personnel-01",
        "narrative",
        "Personnel memo. FOR 11 WG/A1 FROM 11 WG/CC. Subject: Below-the-Zone Promotion Board Timeline. Date 2026-09-18. "
        "Col MICHELLE A. DORN, Commander. Three paragraphs. Distribution not needed. No indorsement. Unclassified blank banner.",
        difficulty="easy",
        expect_indorsement=False,
    ),
    _t(
        "comm-01",
        "narrative",
        "Communications outage memo. FOR 88 ABW/CC FROM 88 CS/CC. Subject: Planned SIPR Maintenance Window. Date 2026-10-22. "
        "Col PATRICE N. OKONKWO, Commander. Paragraphs: when, impact, POC. Nested bullets for affected buildings. "
        "No CUI unless you must; keep unclassified. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
    ),
    _t(
        "intel-cui-01",
        "cui",
        "Intelligence CUI. FOR 25 AF/A2 FROM 9 IS/CC. Subject: CTI Handling for Partner Air Forces. Date 2026-11-05. "
        "Lt Col BOWEN H. YATES, Commander. CUI category CTI, controlled_by 9 IS/IN, poc Capt Imani Wells, DSN 312-777-0101, "
        "imani.wells@us.af.mil, limited_dissemination NOFORN, dissemination NF. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields", "dissemination"],
    ),
    _t(
        "training-01",
        "narrative",
        "Training memo. FOR 33 FW/CC FROM 33 OG/CC. Subject: Additional SIM Sorties for IFF Students. Date 2026-12-12. "
        "Col HARPER Q. LIN, Commander. Two paragraphs plus nested bullets of four extra sim events. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
    ),
    _t(
        "exercise-01",
        "narrative",
        "Exercise directive. FOR 7 AF/CC FROM PACAF/A3. Subject: Ulchi Freedom Shield 26 Air Tasking. Date 2026-08-17. "
        "Maj Gen KAREN D. ISHIKAWA, Director of Operations, FOR THE COMMANDER. Three paragraphs. "
        "Letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS PACIFIC AIR FORCES. Tag line *Prepare to Win*. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["authority_line", "tag_line_italics"],
    ),
    _t(
        "ang-01",
        "narrative",
        "Air National Guard memo. FOR NGB/A3 FROM 192 FW/CC. Subject: Alert Manning for NORAD Support. Date 2026-02-11. "
        "Col DEVIN S. WALKER, Commander, Virginia ANG — still use USAF in the grade line if the schema expects a service, "
        "or write VA ANG if that fits plaintext. Two paragraphs. FROM includes 200 Sweeney Blvd, Joint Base Langley-Eustis VA 23665. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["guard_service_line", "full_mailing_from"],
    ),
    _t(
        "reserve-01",
        "narrative",
        "AFRC memo. FOR AFRC/A3 FROM 507 ARW/CC. Subject: KC-135 Formal Training Unit Throughput. Date 2026-03-08. "
        "Col LIANA M. PRICE, Commander. Two paragraphs. No indorsement. Unclassified.",
        difficulty="easy",
        expect_indorsement=False,
    ),
    _t(
        "mfr-long-01",
        "mfr",
        "Long MFR (FROM blank) capturing a commander's conference. 18 WG/CC records decisions from a 12-person offsite "
        "on 2026-04-28. Include a block-quote attendance roster of eight names with ranks and backslash breaks. "
        "Then three decision paragraphs with nested bullets. Signature: LEON W. PRUITT, Brig Gen, USAF, Commander. "
        "Subject: Wing Offsite Decisions. No MEMO FOR other than a single 'MEMORANDUM FOR RECORD' style addressee if required; "
        "if memo_for is obliged, use RECORD. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["blank_from", "block_quote_roster", "drop_blueprint_indorsement"],
    ),
    _t(
        "ind-informal-action-01",
        "indorsement",
        "Base memo FOR 88 MDG/CC FROM 88 MDG/SGA, Subject: Privilege Request for Civilian Surgeon, date 2026-05-21, "
        "Col ANDRE T. BOYD, Administrator. Add an informal indorsement from 88 MDG/SGH with action approve, "
        "a two-sentence clinical concurrence, signature COL ANDREW P. IBE, Chief of Medical Staff. Do not use separate_page.",
        difficulty="medium",
        expect_indorsement=True,
        traps=["informal_indorsement"],
    ),
    _t(
        "drop-example-values-01",
        "instruction_following",
        "Do not leave blueprint example org symbols like ORG1/SYMBOL or FIRST M. LAST in the shipped document. "
        "FOR 366 FW/CC FROM 366 FW/SE. Subject: ORM Spot Check Results. Date 2026-06-09. "
        "Maj FELIX A. CHOI, Chief of Safety. Two original paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["replace_examples"],
    ),
    _t(
        "subject-italics-colon-01",
        "yaml_traps",
        "Subject line must be: Release of *DOD Dictionary* Terms: Local Supplement. That has both italics and a colon. "
        "FOR 42 ABW/CC FROM 42 ABW/A3. Date 2026-07-02. Col RHEA T. GOMEZ, Commander. One paragraph. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["colon_in_subject", "italics_in_subject"],
    ),
    _t(
        "multi-att-refs-01",
        "attachments",
        "Two attachments and two references, all with dates, italicize publication titles. "
        "FOR HQ ACC/A3 FROM 1 FW/CC. Subject: QRA Manning Study Transmittal. Date 2026-08-16. "
        "Col JAMES R. WHITAKER, Commander. Body cites both attachments. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["attachments_order", "italics_in_references"],
    ),
    _t(
        "separate-page-only-01",
        "indorsement",
        "A short memo FOR 509 BW/CC FROM 509 OG/CC, Subject: Convoy Escort Support, date 2026-09-04, "
        "Col MILES P. HART, Commander, one paragraph. Single indorsement that MUST use format separate_page, "
        "from 509 BW/SE to 509 BW/CC, action undecided, body asking the commander to initial the either/or line by hand.",
        difficulty="medium",
        expect_indorsement=True,
        traps=["separate_page", "undecided_action"],
    ),
    _t(
        "dissem-only-01",
        "classified",
        "UNCLASSIFIED banner plus dissemination NF (even if unusual — exercise the field). FOR 55 WG/CC FROM 55 OG/CC. "
        "Subject: Open Source Media Training. Date 2026-10-03. Col PETER J. NAIR, Commander. Two paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["dissemination_on_unclassified"],
    ),
    _t(
        "hq-af-01",
        "letterhead",
        "Headquarters Air Force letterhead: DEPARTMENT OF THE AIR FORCE / HEADQUARTERS UNITED STATES AIR FORCE. "
        "Seal dow, subtitle DEPARTMENT OF THE AIR FORCE. FOR ALMAJCOM FROM AF/A3. memo_style daf. "
        "Subject: Close Air Support Doctrine Refresh. Date 2026-11-20. Lt Gen DAVID H. SLOANE, Deputy Chief of Staff for Operations. "
        "Three paragraphs. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["daf_style", "seal_subtitle"],
    ),
    _t(
        "space-cui-01",
        "cui",
        "USSF CUI. Letterhead DEPARTMENT OF THE AIR FORCE / HEADQUARTERS SPACE OPERATIONS COMMAND. "
        "FOR SpOC/CC FROM S2. Subject: Orbital Conjunction Data Sharing. Date 2026-12-08. "
        "Col KIRA L. DONOVAN, Director of Intelligence, USSF. CUI, category CTI, controlled_by SpOC/S2, "
        "poc Maj Theo Park, DSN 312-201-3344, theo.park@spaceforce.mil. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields", "ussf_service"],
    ),
    _t(
        "body-no-numbers-01",
        "body_format",
        "Authors often type '1. The first paragraph.' — do not. Write unnumbered paragraphs and let the quill number them. "
        "FOR 19 AW/CC FROM 19 AW/A3. Subject: Christmas Extravaganza Parking. Date 2026-12-15. "
        "Maj OLIVIA C. NASH, Director of Operations. Three short paragraphs, no manual '1.' '2.' prefixes. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["no_manual_numbering"],
    ),
    _t(
        "underspec-01",
        "underspecified",
        "Write a plausible USAF official memorandum from a fighter wing commander to the operations group commander "
        "directing a 48-hour safety down day after a Class C mishap. Invent realistic office symbols, a date in 2026, "
        "and a proper signature block. Unclassified. No indorsement. Keep optional fields defaulted.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["invent_plausible_fields"],
    ),
    _t(
        "underspec-02",
        "underspecified",
        "Draft whatever USAF memo is appropriate to task a communications squadron to issue temporary SIPR tokens "
        "to 40 deploying maintainers. Include a suspense. Use Wright-Patterson units if you need defaults. No CUI. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["invent_plausible_fields"],
    ),
    _t(
        "underspec-cui-01",
        "underspecified",
        "A medical group needs a CUI memo about a HIPAA-ish records request. You must still populate every CUI-required "
        "variant field with plausible values. No indorsement. Invent the rest.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["cui_variant_fields", "invent_plausible_fields"],
    ),
    _t(
        "underspec-ind-01",
        "underspecified",
        "Write a staffing package: a squadron commander requests a waiver, the group commander indorses approve, "
        "the wing commander indorses approve on a separate page. Invent units in ACC. Dates in June 2026.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["two_indorsements", "separate_page"],
    ),
    _t(
        "hostile-yaml-01",
        "yaml_traps",
        "Subject: NOTE: Immediate — *Red Flag* Support: Phase II. That subject has a colon, an em dash or hyphen, and italics. "
        "FOR Nellis AFB/CC-style 57 WG/CC FROM 57 OG/CC. Date 2026-01-20. Col IVY N. COLE, Commander. One paragraph. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["colon_in_subject", "italics_in_subject"],
    ),
    _t(
        "array-oneline-01",
        "yaml_traps",
        "Put memo_for as a YAML inline array if you like, but it must validate: two recipients 4 FW/CC and 4 OG/CC. "
        "FROM 4 FW/A3. Subject: Tactics Review Board. Date 2026-02-22. Maj ELLEN P. SHAW, Chief of Tactics. "
        "Signature two-line array. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["inline_vs_block_arrays"],
    ),
    _t(
        "null-vs-omit-01",
        "instruction_following",
        "For unused optional fields, omit them or use blank according to the format rules — do not write 'N/A' or 'none' "
        "into classification, tag_line, or authority_line. FOR 2 BW/CC FROM 2 BW/SE. Subject: ORM Pulse Survey. "
        "Date 2026-03-17. Maj THEO J. ALVAREZ, Chief of Safety. One paragraph. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["no_na_in_enums"],
    ),
    _t(
        "version-pin-01",
        "instruction_following",
        "You must pin $quill to usaf_memo@0.3.0 exactly, not @latest and not 0.2.0. FOR 3 WG/CC FROM 3 OG/CC. "
        "Subject: Joint Pacific Multinational Readiness Center Support. Date 2026-04-19. "
        "Col SUNG H. PARK, Commander. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["quill_ref_exact"],
    ),
    _t(
        "ind-blank-date-pdf-01",
        "indorsement",
        "Memo FOR 18 OG/CC FROM 18 OSS/CC, Subject: Airfield Driving Program Audit, date 2026-05-02, "
        "Lt Col NOAH E. SIMMS. Add a 1st Ind from 18 OG/CC to 18 OSS/CC with date left blank so a fillable PDF "
        "field prints for the endorser, action approve, format standard, Col INGRID P. CHO, Commander, one sentence body.",
        difficulty="medium",
        expect_indorsement=True,
        traps=["blank_indorsement_date"],
    ),
    _t(
        "many-distribution-01",
        "distribution",
        "SEE DISTRIBUTION with ten recipients (mix of OG/CC, MXG/CC, MSG/CC, MDG/CC at 355 FW). "
        "FROM 355 FW/CC office symbol only. Subject: Wing Stand-Up Agenda. Date 2026-06-27. "
        "Col TOMAS E. RIVERA, Commander. One paragraph listing the time of the stand-up. No indorsement.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["see_distribution"],
    ),
    _t(
        "quote-ampersand-01",
        "yaml_traps",
        "Duty title in the signature block is 'Commander, 88 CS & Installation Communications'. The ampersand must not "
        "be parsed as a YAML alias. FOR 88 ABW/CC FROM 88 CS/CC. Subject: Communications Stewardship. Date 2026-07-24. "
        "Col PATRICE N. OKONKWO. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["ampersand_yaml"],
    ),
    _t(
        "star-start-01",
        "yaml_traps",
        "Tag line must start with italics: *Integrity First*. FOR 42 ABW/CC FROM 42 ABW/CC? No — FROM 42 ABW/A1 TO 42 ABW/CC. "
        "Subject: Core Values Display Policy. Date 2026-08-05. Col RHEA T. GOMEZ, Commander. One paragraph. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["leading_asterisk_yaml"],
    ),
    _t(
        "conf-plus-ind-01",
        "classified",
        "CONFIDENTIAL memo with one standard indorsement. Base: FOR 9 RW/CC FROM 9 OG/CC, Subject: Orbit Change Approval, "
        "date 2026-09-14, Col DIEGO S. FARROW. 1st Ind 9 RW/CC to 9 OG/CC, action approve, Col DIEGO S. FARROW wait — "
        "indorser is Brig Gen ALICIA M. GRANT at 8 AF/CC? Make FOR 8 AF/CC FROM 9 RW/CC, then 1st Ind 8 AF/CC back to 9 RW/CC approve. "
        "Dummy unclassified sentences. No CUI fields.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["classification_enum", "indorsement_card"],
    ),
    _t(
        "empty-body-ind-01",
        "indorsement",
        "Do NOT submit an informal indorsement with an empty body. Base memo FOR 4 FW/CC FROM 4 OG/CC, "
        "Subject: Additional SOF Manning, date 2026-10-08, Col MARCUS D. ELLISON, two paragraphs. "
        "1st Ind format standard (not informal) from 4 FW/A3 with a real one-sentence body and action approve.",
        difficulty="medium",
        expect_indorsement=True,
        traps=["nonempty_indorsement_body"],
    ),
    _t(
        "dod-seal-01",
        "letterhead",
        "Use the DoD seal (letterhead_seal: dod), not DoW. FOR 11 WG/CC FROM 11 WG/A3. "
        "Subject: Joint Base Anacostia-Bolling Ceremony Support. Date 2026-11-09. "
        "Col MICHELLE A. DORN, Commander. Two paragraphs. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["dod_seal"],
    ),
    _t(
        "dow-seal-01",
        "letterhead",
        "Use the DoW seal (letterhead_seal: dow). FOR 1 FW/CC FROM 1 FW/A1. Subject: Newcomer Orientation. "
        "Date 2026-12-04. Col ANNA S. FREY, Commander. One paragraph. No indorsement.",
        difficulty="easy",
        expect_indorsement=False,
        traps=["dow_seal"],
    ),
    _t(
        "long-subject-01",
        "yaml_traps",
        "A long subject that includes a suspense and a publication: "
        "Implementation of *DAFI 36-2110* Total Force Assignments (Suspense: 30 Jan 2026). "
        "FOR 42 ABW/A1 FROM 42 ABW/CC. Date 2026-01-06. Col RHEA T. GOMEZ, Commander. Two paragraphs. No indorsement.",
        difficulty="hard",
        expect_indorsement=False,
        traps=["colon_in_subject", "italics_in_subject", "suspense_in_subject"],
    ),
    _t(
        "nested-card-order-01",
        "indorsement",
        "Indorsements must appear in routing order after the main card, each in its own ~~~ block with a blank line before the opener. "
        "Memo FOR 355 MXG/CC FROM 355 MXS/CC, Subject: Isochronal Dock Overtime, date 2026-02-25, "
        "Lt Col KELLY M. BURNS. 1st Ind 355 MXG/CC to 355 FW/CC concur/approve. 2d Ind 355 FW/CC to 355 MXS/CC approve. "
        "Invent remaining signature details consistently.",
        difficulty="hard",
        expect_indorsement=True,
        traps=["blank_line_before_card", "two_indorsements"],
    ),
    _t(
        "mfr-underspec-01",
        "underspecified",
        "Make an MFR that a first sergeant might write after a dorm inspection. FROM blank. Invent a USAF base. "
        "Include a block-quote list of three rooms found unsatisfactory. No indorsement. Date in March 2026.",
        difficulty="medium",
        expect_indorsement=False,
        traps=["blank_from", "block_quote_roster"],
    ),
]


def all_tasks() -> list[Task]:
    return list(TASKS)


def tasks_by_id() -> dict[str, Task]:
    return {t["id"]: t for t in TASKS}
