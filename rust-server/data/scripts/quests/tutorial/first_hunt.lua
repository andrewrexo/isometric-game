-- First Hunt Quest Script
-- Handles branching dialogue and quest logic

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
        speaker = "Village Elder",
        text = "The slimes grow bolder each day, threatening our village. Will you help us?",
        choices = {
            { id = "accept", text = "I'll handle it." },
            { id = "decline", text = "Not right now." },
            { id = "ask_reward", text = "What's in it for me?" }
        }
    })

    if choice == "accept" then
        ctx:accept_quest()
        ctx:show_dialogue({
            speaker = "Village Elder",
            text = "Bless you! Slay 5 slimes and bring me 3 of their cores. Be careful out there."
        })
    elseif choice == "ask_reward" then
        ctx:show_dialogue({
            speaker = "Village Elder",
            text = "50 gold coins, some healing potions, and the gratitude of our village. A fair deal?"
        })
        return show_offer_dialogue(ctx)  -- Loop back to choice
    else
        ctx:show_dialogue({
            speaker = "Village Elder",
            text = "I understand. Come back when you're ready to help."
        })
    end
end

-- Show progress dialogue
function show_progress_dialogue(ctx)
    local slimes = ctx:get_objective_progress("kill_slimes")
    local cores = ctx:get_objective_progress("collect_cores")

    local text = string.format(
        "How goes the hunt? You've slain %d of 5 slimes and collected %d of 3 cores.",
        slimes.current, cores.current
    )

    ctx:show_dialogue({
        speaker = "Village Elder",
        text = text
    })
end

-- Called when an objective is updated
function on_objective_progress(ctx, objective_id, new_count)
    if objective_id == "kill_slimes" and new_count == 5 then
        ctx:show_notification("All slimes defeated! Collect their cores.")
    elseif objective_id == "collect_cores" and new_count == 3 then
        ctx:show_notification("Cores collected! Return to the Village Elder.")
    end
end

-- Complete the quest with bonus check
function complete_quest(ctx)
    -- Complete the quest FIRST (grants rewards, updates state)
    ctx:complete_quest()

    -- Unlock next quest in chain
    ctx:unlock_quest("forest_dangers")

    -- Check for speed bonus
    local duration = ctx:get_quest_duration()

    if duration < 300 then  -- Under 5 minutes
        ctx:grant_bonus_reward({ gold = 25 })
        ctx:show_dialogue({
            speaker = "Village Elder",
            text = "Incredible speed! The slimes didn't stand a chance. Take this extra reward! When you're ready for another task, speak with me again."
        })
    else
        ctx:show_dialogue({
            speaker = "Village Elder",
            text = "Wonderful work! The village is safer thanks to you. When you're ready for another task, speak with me again."
        })
    end
end

-- Post-completion dialogue
function show_completed_dialogue(ctx)
    ctx:show_dialogue({
        speaker = "Village Elder",
        text = "Thank you again for your help with the slimes. Speak with me when you're ready for more work."
    })
end
