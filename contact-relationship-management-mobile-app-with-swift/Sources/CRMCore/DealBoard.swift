// Deals grouped by pipeline and stage, in stage order, as the board in the web UI shows them.
import Foundation

public struct DealColumn: Sendable, Identifiable, Equatable {
    public let stage: StageRecord
    public let deals: [Deal]
    public var id: UUID { stage.pid }
}

public struct DealBoard: Sendable, Identifiable, Equatable {
    public let pipeline: PipelineRecord
    public let columns: [DealColumn]
    public var id: UUID { pipeline.pid }

    /// One board per pipeline. Within a pipeline the stages are in `position` order and the deals in `kanbanPosition`
    /// order, then by name. A deal whose stage is not in its pipeline is left out of the columns and counted in
    /// `unplaced`, so the caller can say so rather than lose it silently.
    public static func boards(pipelines: [Pipeline], deals: [Deal]) -> (boards: [DealBoard], unplaced: [Deal]) {
        var byStage: [UUID: [Deal]] = [:]
        for deal in deals { byStage[deal.stagePid, default: []].append(deal) }
        var placed = Set<UUID>()
        let boards = pipelines.map { pipeline in
            let stages = pipeline.stages
                .filter { $0.pipelinePid == pipeline.pipeline.pid }
                .sorted { ($0.position, $0.name) < ($1.position, $1.name) }
            let columns = stages.map { stage in
                let inStage = (byStage[stage.pid] ?? [])
                    .filter { $0.pipelinePid == pipeline.pipeline.pid }
                    .sorted { ($0.kanbanPosition, $0.name) < ($1.kanbanPosition, $1.name) }
                placed.formUnion(inStage.map(\.pid))
                return DealColumn(stage: stage, deals: inStage)
            }
            return DealBoard(pipeline: pipeline.pipeline, columns: columns)
        }
        return (boards, deals.filter { !placed.contains($0.pid) })
    }
}
