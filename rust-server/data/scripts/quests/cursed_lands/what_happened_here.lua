-- What Happened Here Quest Script
-- First quest in the Cursed Lands chain

-- Called when player interacts with quest giver NPC
function on_interact(ctx)
    local quest_state = ctx:get_quest_state()

    if quest_state == "not_started" then
        return show_offer_dialogue(ctx)
    elseif quest_state == "in_progress" then
        return show_progress_dialogue(ctx)
    elseif quest_state == "ready_to_complete" then
        return complete_quest(ctx)
    elseif quest_state == "completed" then
        return show_completed_dialogue(ctx)
    end
end

-- Show quest offer with choices
function show_offer_dialogue(ctx)
    local choice = ctx:show_dialogue({
        speaker = "Elder Mara",
        text = "You survived... thank the light. The corruption came without warning. The pigs... they've changed. Their eyes glow with that terrible corruption. Will you help us understand what happened?",
        choices = {
            { id = "accept", text = "I'll investigate." },
            { id = "decline", text = "I need to prepare first." },
            { id = "ask_more", text = "What do you need me to do?" }
        }
    })

    if choice == "accept" then
        ctx:accept_quest()
        ctx:show_dialogue({
            speaker = "Elder Mara",
            text = "Be careful out there. Kill the corrupted pigs and bring me their tainted meat - it will tell us much about this corruption. Return when you've gathered enough evidence."
        })
    elseif choice == "ask_more" then
        ctx:show_dialogue({
            speaker = "Elder Mara",
            text = "Slay 15 of the corrupted pigs and collect 10 pieces of their spoiled meat. The meat may hold clues about the source of this corruption."
        })
        return show_offer_dialogue(ctx)
    else
        ctx:show_dialogue({
            speaker = "Elder Mara",
            text = "I understand. The corruption will still be here when you're ready. Be safe."
        })
    end
end

-- Show progress dialogue
function show_progress_dialogue(ctx)
    local pigs = ctx:get_objective_progress("kill_corrupted_pigs")
    local meat = ctx:get_objective_progress("collect_spoiled_meat")

    local text = string.format(
        "Have you learned anything about the corruption? You've slain %d of 15 corrupted pigs and collected %d of 10 spoiled meat.",
        pigs.current, meat.current
    )

    ctx:show_dialogue({
        speaker = "Elder Mara",
        text = text
    })
end

-- Called when an objective is updated
function on_objective_progress(ctx, objective_id, new_count)
    if objective_id == "kill_corrupted_pigs" and new_count == 15 then
        ctx:show_notification("All corrupted pigs slain! Collect their meat.")
    elseif objective_id == "collect_spoiled_meat" and new_count == 10 then
        ctx:show_notification("Evidence gathered! Return to Elder Mara.")
    end
end

-- Complete the quest
function complete_quest(ctx)
    ctx:show_dialogue({
        speaker = "Elder Mara",
        text = "This meat... the corruption runs deep. But you've proven yourself capable. Take this blade - I salvaged it from the ruins. You'll need it for what lies ahead."
    })

    ctx:complete_quest()

    -- Unlock next quest in chain
    ctx:unlock_quest("spreading_rot")
end

-- Post-completion dialogue
function show_completed_dialogue(ctx)
    ctx:show_dialogue({
        speaker = "Elder Mara",
        text = "You've done well. Perhaps there is hope for us yet. Speak with me when you're ready for more work - the corruption spreads further each day."
    })
end
