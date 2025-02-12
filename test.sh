#!/usr/bin/env bash

# # one hour left, two activities required. should pop up twice
# cargo r -- test --work-duration 00:25 --break-duration 00:05 --program-start 12:30 --periods 3 --window 12:00..13:30 --activity 'drink tea:2'

# # 4 breaks only 2 activities, should not pop up then pop up then not and then pop up 
# cargo r -- test --work-duration 00:25 --break-duration 00:20 --program-start 12:00 --periods 4 --window 12:00..14:01 --activity 'drink tea:2' --activity '!relax'

# # 4 breaks only 2 activities, should not pop if firefox is open
# cargo r -- test --work-duration 00:25 --break-duration 00:20 --program-start 12:00 --periods 4 --window 12:00..14:01 --activity 'drink tea:2' --activity '!relax' --skip-when-visible "firefox"

# 4 breaks only 2 activities, should not pop if firefox is open
cargo r -- test --work-duration 00:25 --break-duration 00:05 --program-start 12:00 --periods 8 --window 12:00..16:01 --activity 'drink tea:2'  --skip-when-visible "firefox"
