package io.github.contactrelationshipmanagement.mobile.core

/** One stage and its deals, in board order. */
public data class StageColumn(val stage: StageRecord, val deals: List<Deal>)

/** One pipeline and its stages, in stage order. */
public data class PipelineBoard(val pipeline: PipelineRecord, val columns: List<StageColumn>)

public object DealBoard {
    /**
     * Groups [deals] by pipeline and stage: pipelines in the order given, stages by `position`, deals by
     * `kanban_position` then name. A deal whose stage is not in any pipeline is returned in [Grouped.unplaced], so
     * nothing is dropped silently.
     */
    public fun group(pipelines: List<Pipeline>, deals: List<Deal>): Grouped {
        val byStage = deals.groupBy { it.stagePid }
        val known = mutableSetOf<String>()
        val boards = pipelines.map { pipeline ->
            val columns = pipeline.stages.sortedBy { it.position }.map { stage ->
                known += stage.pid
                StageColumn(stage, byStage[stage.pid].orEmpty().sortedWith(compareBy({ it.kanbanPosition }, { it.name })))
            }
            PipelineBoard(pipeline.pipeline, columns)
        }
        return Grouped(boards, deals.filter { it.stagePid !in known })
    }

    public data class Grouped(val boards: List<PipelineBoard>, val unplaced: List<Deal>)
}
