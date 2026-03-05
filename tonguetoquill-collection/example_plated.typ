#import "@preview/ttq-classic-resume:0.1.0": *

#show: resume

#header_block(
  name: json(bytes("\"John Doe\"")),
  contacts: json(bytes("[\"john.doe@example.com\",\"(555) 123-4567\",\"github.com/johndoe\",\"linkedin.com/in/johndoe\",\"Pittsburgh, PA\"]")),
)


  
    #section_header(json(bytes("\"Active Certifications\"")))
  

  
    #simple_grid(items: json(bytes("[\"Offensive Security Certified Professional (OSCP)\",\"GIAC Cyber Threat Intelligence (GCTI)\",\"CompTIA CASP+, CySA+, Sec+, Net+, A+, Proj+\",\"GIAC Machine Learning Engineer (GMLE)\"]")))
  

  
    #section_header(json(bytes("\"Skills\"")))
  

  
    #key_value_grid(items: (
      (key: json(bytes("\"Programming\"")), value: json(bytes("\"Python, R, JS, C#, Rust, PowerShell, CI/CD\""))),
      (key: json(bytes("\"Data Science\"")), value: json(bytes("\"ML/statistics, TensorFlow, AI Engineering\""))),
      (key: json(bytes("\"IT & Cybersecurity\"")), value: json(bytes("\"AD DS, Splunk, Metasploit, Wireshark, Nessus\""))),
      (key: json(bytes("\"Cloud\"")), value: json(bytes("\"AWS EC2/S3, Helm, Docker, Serverless\""))),
    ))
  

  
    #section_header(json(bytes("\"Work Experience\"")))
  

  
    #entry_block(
      headingLeft: json(bytes("\"Templar Archives Research Division\"")),
      headingRight: json(bytes("\"August 2024 – Present\"")),
      subheadingLeft: json(bytes("\"Psionic Research Analyst\"")),
      subheadingRight: json(bytes("\"Aiur\"")),
      body: eval("- Analyzed Khala disruption patterns following Amon's corruption, developing countermeasures to protect remaining neural link infrastructure.\n- Building automated threat detection pipelines using Khaydarin crystal arrays to monitor Void energy signatures across the sector.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"Terran Dominion Ghost Academy\"")),
      headingRight: json(bytes("\"May 2025 – July 2025\"")),
      subheadingLeft: json(bytes("\"Covert Ops Trainee\"")),
      subheadingRight: json(bytes("\"Tarsonis (Remote)\"")),
      body: eval("- Developed tactical HUD displays for Ghost operatives integrating real-time Zerg hive cluster intelligence.\n- Created automated target acquisition systems for nuclear launch protocols; involved cloaking field calibration and EMP targeting.\n- Discovered (and reported) a critical vulnerability in Adjutant defense networks exploitable by Zerg Infestors.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"Abathur's Evolution Pit\"")),
      headingRight: json(bytes("\"June 2023 – July 2023\"")),
      subheadingLeft: json(bytes("\"Biomass Research Intern\"")),
      subheadingRight: json(bytes("\"Char\"")),
      body: eval("- Developed tracking algorithms for Overlord surveillance networks; supported pattern-of-life analysis for Terran outpost elimination.\n- Prototyped a creep tumor optimization tool featuring swarm pathfinding, resource node mapping, and hatchery placement recommendations.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"Raynor's Raiders\"")),
      headingRight: json(bytes("\"January 2018 – June 2020\"")),
      subheadingLeft: json(bytes("\"Combat Engineer\"")),
      subheadingRight: json(bytes("\"Mar Sara\"")),
      body: eval("- Administered Hyperion shipboard systems, SCV maintenance protocols, and bunker defense automation for 30,000+ colonists.\n- Developed siege tank targeting scripts, delivered Zerg threat briefs, and integrated supply depot optimization procedures.\n- Achieved Distinguished Graduate honors at the Mar Sara Militia Academy.\n- Awarded the Raynor's Star and Mar Sara Defense Medal for meritorious service against the Swarm.\n\n", mode: "markup"),
    )
  

  
    #section_header(json(bytes("\"Education\"")))
  

  
    #entry_block(
      headingLeft: json(bytes("\"Carnegie Mellon University\"")),
      headingRight: json(bytes("\"December 2025\"")),
      subheadingLeft: json(bytes("\"Master of Information Technology Strategy\"")),
      subheadingRight: json(bytes("\"Pittsburgh, PA\"")),
      body: eval("", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"United States Air Force Academy\"")),
      headingRight: json(bytes("\"May 2024\"")),
      subheadingLeft: json(bytes("\"BS, Data Science\"")),
      subheadingRight: json(bytes("\"Colorado Springs, CO\"")),
      body: eval("- Distinguished Graduate (top 10%); Chinese language minor (L2+/R1 on DLPT).\n- Delogrand deputy captain, cyber combat lead, and web exploit SME.\n- Professor Bradley A. Warner Data Science Catalyst and Top Cadet in Computer Networks.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"Western Governors University\"")),
      headingRight: json(bytes("\"April 2022\"")),
      subheadingLeft: json(bytes("\"BS, Cybersecurity and Information Assurance\"")),
      subheadingRight: json(bytes("\"Remote\"")),
      body: eval("", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"Community College of the Air Force\"")),
      headingRight: json(bytes("\"February 2019\"")),
      subheadingLeft: json(bytes("\"AS, Information Systems Technology\"")),
      subheadingRight: json(bytes("\"Remote\"")),
      body: eval("", mode: "markup"),
    )
  

  
    #section_header(json(bytes("\"Cyber Competition\"")))
  

  
    #entry_block(
      headingLeft: json(bytes("\"1st in SANS Academy Cup 2024\"")),
      headingRight: json(bytes("\"\"")),
      subheadingLeft: json(bytes("\"\"")),
      subheadingRight: json(bytes("\"\"")),
      body: eval("- Competed as the Delogrand Web Exploit SME, solving SQLi, API, and HTTP packet crafting problems.\n- Also placed first in SANS Core Netwars competition.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"1st in NCX 2023\"")),
      headingRight: json(bytes("\"\"")),
      subheadingLeft: json(bytes("\"\"")),
      subheadingRight: json(bytes("\"\"")),
      body: eval("- Developed strategies, defensive scripts, and exploits for the Cyber Combat event.\n- Analyzed logs with Bash and Python for the Data Analysis event.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"1st in SANS Academy Cup 2023\"")),
      headingRight: json(bytes("\"\"")),
      subheadingLeft: json(bytes("\"\"")),
      subheadingRight: json(bytes("\"\"")),
      body: eval("- Competed as the Delogrand Web Exploit SME, solving XSS, XXE, SQLi, and HTTP crafting problems.\n- Took first place against rival Army, Navy, and Coast Guard service academy teams.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"1st in RMCS 2023\"")),
      headingRight: json(bytes("\"\"")),
      subheadingLeft: json(bytes("\"\"")),
      subheadingRight: json(bytes("\"\"")),
      body: eval("- Competed as the Delogrand Web Exploit SME, solving obfuscated JS, Wasm, XSS, and SQLi problems.\n\n", mode: "markup"),
    )
  

  
    #entry_block(
      headingLeft: json(bytes("\"1st in NCX 2022\"")),
      headingRight: json(bytes("\"\"")),
      subheadingLeft: json(bytes("\"\"")),
      subheadingRight: json(bytes("\"\"")),
      body: eval("- Trained and strategized teams for the Cyber Combat event.\n\n", mode: "markup"),
    )
  

  
    #section_header(json(bytes("\"Projects\"")))
  

  
    #project_entry(
      name: json(bytes("\"TongueToQuill\"")),
      url: json(bytes("\"https://www.tonguetoquill.com\"")),
      body: eval("- Rich markdown editor for perfectly formatted USAF and USSF documents with Claude MCP integration.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"Quillmark\"")),
      url: json(bytes("\"https://github.com/nibsbin/quillmark\"")),
      body: eval("- Parameterization engine for generating arbitrarily typesetted documents from markdown content.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"RoboRA\"")),
      url: json(bytes("\"https://github.com/nibsbin/RoboRA\"")),
      body: eval("- AI research automation framework for Dr. Nadiya Kostyuk's research on global cyber policy.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"Scraipe\"")),
      url: json(bytes("\"https://pypi.org/project/scraipe/\"")),
      body: eval("- An asynchronous scraping and enrichment library to automate cybersecurity research.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"Quandry\"")),
      url: json(bytes("\"https://quandry.streamlit.app/\"")),
      body: eval("- LLM Expectation Engine to automate security and behavior evaluation of LLM models.\n- Awarded 1st place out of 11 teams in CMU's Fall 2024 Information Security, Privacy, and Policy poster fair.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"Streamlit Scroll Navigation\"")),
      url: json(bytes("\"https://pypi.org/project/streamlit-scroll-navigation/\"")),
      body: eval("- Published a Streamlit-featured PyPI package to help data scientists create fluid single-page applications.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"ADSBLookup\"")),
      url: json(bytes("\"<closed source>\"")),
      body: eval("- Reversed the internal API of a popular ADSB web service to pull comprehensive live ADSB datasets; ported and exposed attributes in a user-friendly, Pandas-compatible Python library for data scientists.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"OSCP LaTeX Report Template\"")),
      url: json(bytes("\"https://github.com/SnpM/oscp-latex-report-template\"")),
      body: eval("- Published a report template that features custom commands for streamlined penetration test documentation.\n\n", mode: "markup"),
    )
  

  
    #project_entry(
      name: json(bytes("\"Lockstep Framework\"")),
      url: json(bytes("\"https://github.com/SnpM/LockstepFramework\"")),
      body: eval("- As a budding programmer, I created a popular RTS engine with custom-built deterministic physics.\n\n", mode: "markup"),
    )
  
