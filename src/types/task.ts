export type TaskStatus = 'PENDING' | 'COMPLETED' | 'FAILED';

export interface PipelineTask {
  id: string;
  game_id: string;
  task_type: string;
  status: TaskStatus;
  target_id: string | null;
  created_at: string;
  updated_at: string;
}
