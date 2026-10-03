@rem khor shell integration for cmd.exe (written, not wired yet: khor does
@rem not host on Windows; see docs/SHELL-INTEGRATION.md).
@rem
@rem cmd has no hook before a command runs, so only the prompt marks can be
@rem made, the way Windows Terminal's documentation does it: D (without a
@rem status, which cmd does not expose to PROMPT) and A before the prompt, B
@rem after it. $e is ESC; ESC \ ends each mark.
@prompt $e]133;D$e\$e]133;A$e\$e]7;file://%COMPUTERNAME%/$P$e\$P$G$e]133;B$e\
