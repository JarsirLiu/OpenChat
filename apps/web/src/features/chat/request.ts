import type {
  CatalogModel,
  CatalogTool,
  UploadedImageAttachment,
} from './types'

export const buildChatRequest = (
  sessionId: string,
  prompt: string,
  textModel: CatalogModel | null,
  imageTool: CatalogTool | null,
  attachments: UploadedImageAttachment[],
) => {
  return {
    session_id: sessionId,
    prompt,
    ...(attachments.length > 0 ? { attachments } : {}),
    ...(textModel
        ? {
            text_model: {
              model_config_id: textModel.model_config_id,
              model: textModel.model,
              provider: textModel.provider,
              runtime_provider: 'openai_compatible',
              source: textModel.source || undefined,
              display_name: textModel.display_name || undefined,
              type: textModel.type,
              input_modalities: textModel.input_modalities,
            },
          }
      : {}),
    ...(imageTool
      ? {
          tool_list: [
            {
              model_config_id: imageTool.model_config_id,
              id: imageTool.id,
              model: imageTool.model,
              provider: imageTool.provider,
              runtime_provider: 'openai_compatible',
              source: imageTool.source || undefined,
              display_name: imageTool.display_name || undefined,
              type: imageTool.type,
            },
          ],
        }
      : {}),
  }
}
