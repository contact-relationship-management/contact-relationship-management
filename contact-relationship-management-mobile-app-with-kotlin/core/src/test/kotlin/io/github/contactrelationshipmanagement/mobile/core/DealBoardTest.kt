package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals

class DealBoardTest {
    private fun stage(pid: String, position: Int) = StageRecord(pid, pid, "p", position, isWon = false, isLost = false, probabilityPercent = 10)
    private fun deal(name: String, stage: String, position: Int) =
        Deal(name, name, "USD", amountMinorText = "1", pipelinePid = "p", stagePid = stage, kanbanPosition = position, won = false)

    @Test fun stagesInPositionOrderAndDealsInBoardOrder() {
        val pipeline = Pipeline(PipelineRecord("p", "Sales"), listOf(stage("won", 3), stage("new", 1), stage("qualified", 2)))
        val grouped = DealBoard.group(listOf(pipeline), listOf(deal("b", "new", 1), deal("a", "new", 1), deal("c", "new", 0), deal("d", "won", 0)))
        val columns = grouped.boards.single().columns
        assertEquals(listOf("new", "qualified", "won"), columns.map { it.stage.pid })
        assertEquals(listOf("c", "a", "b"), columns[0].deals.map { it.name })
        assertEquals(emptyList(), columns[1].deals)
        assertEquals(emptyList(), grouped.unplaced)
    }

    @Test fun aDealInNoKnownStageIsKeptApart() {
        val grouped = DealBoard.group(emptyList(), listOf(deal("x", "gone", 0)))
        assertEquals(listOf("x"), grouped.unplaced.map { it.name })
    }
}
