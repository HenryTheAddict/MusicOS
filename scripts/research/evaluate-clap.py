"""Development-only model evaluation. Production must not depend on Python.
Run with the isolated out/rebuild-qa/model-evaluation/venv environment.
Downloads public weights anonymously; never reads local music or uploads audio.
"""
import argparse, json, time, resource, platform
from pathlib import Path
import numpy as np
import torch
from huggingface_hub import snapshot_download
from transformers import ClapModel, ClapFeatureExtractor, AutoTokenizer
import onnxruntime as ort
parser=argparse.ArgumentParser()
parser.add_argument('--checkpoint',default='laion/larger_clap_music')
parser.add_argument('--revision',default='a0b4534a14f58e20944452dff00a22a06ce629d1')
parser.add_argument('--output',default='out/rebuild-qa/model-evaluation')
options=parser.parse_args()
ROOT=Path(options.output);ROOT.mkdir(parents=True,exist_ok=True)
REVISION=options.revision
root=snapshot_download(options.checkpoint,revision=REVISION,token=False,local_dir=str(ROOT/'music-checkpoint'))
print('Checkpoint downloaded',flush=True)
torch.set_num_threads(2)
started=time.perf_counter();model,loading=ClapModel.from_pretrained(root,local_files_only=True,output_loading_info=True);model.eval();load_seconds=time.perf_counter()-started
if any(loading.get(key) for key in ['missing_keys','unexpected_keys','mismatched_keys','error_msgs']):raise RuntimeError(f'Checkpoint conversion mismatch: {loading}')
processor=ClapFeatureExtractor.from_pretrained(root,local_files_only=True)
tokenizer=AutoTokenizer.from_pretrained(root,local_files_only=True)
rng=np.random.default_rng(9);audio=rng.normal(0,.05,480000).astype(np.float32)
inputs=processor(audio,sampling_rate=48000,return_tensors='pt')
print({k:tuple(v.shape) for k,v in inputs.items()},flush=True)
class Audio(torch.nn.Module):
    def __init__(self):super().__init__();self.audio=model.audio_model;self.projection=model.audio_projection
    def forward(self,input_features):
        output=self.audio(input_features=input_features,return_dict=False)[1]
        embedding=self.projection(output)
        return torch.nn.functional.normalize(embedding,dim=-1)
wrapper=Audio().eval();features=inputs['input_features']
with torch.no_grad():
    expected=wrapper(features).numpy();times=[]
    for _ in range(3):
        start=time.perf_counter();wrapper(features);times.append(time.perf_counter()-start)
    prompts=[f'Music that sounds {mood}.' for mood in ['chilled','energetic','melancholic','bright','focused','dreamy']]
    text=tokenizer(prompts,padding=True,return_tensors='pt')
    mood=model.get_text_features(**text);mood=torch.nn.functional.normalize(mood,dim=-1).numpy()
(ROOT/'mood-vectors.json').write_text(json.dumps(dict(zip(['Chilled','Energetic','Melancholic','Bright','Focused','Dreamy'],mood.tolist()))))
(ROOT/'mood-cosines.json').write_text(json.dumps((mood@mood.T).tolist(),indent=2)+'\n')
print('PyTorch inference seconds',times,flush=True)
report={'checkpoint':options.checkpoint,'revision':REVISION,'license':'Verify exact checkpoint model card separately','download_bytes':(Path(root)/'pytorch_model.bin').stat().st_size,'platform':platform.platform(),'torch':torch.__version__,'onnxruntime':ort.__version__,'load_seconds':load_seconds,'torch_seconds_per_10s':times,'peak_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'input_shape':list(features.shape),'listening_evaluation':'Not performed; synthetic input tests runtime correctness only.'}
try:
    output=ROOT/'music-audio.onnx'
    with torch.no_grad():torch.onnx.export(wrapper,(features,),str(output),input_names=['input_features'],output_names=['embedding'],opset_version=17,dynamo=False)
    session=ort.InferenceSession(str(output),providers=['CPUExecutionProvider'])
    actual=session.run(None,{'input_features':features.numpy()})[0]
    report['onnx_bytes']=output.stat().st_size
    report['onnx_max_abs_error']=float(np.abs(actual-expected).max())
    report['onnx_cosine']=float((actual*expected).sum())
    times=[]
    for _ in range(3):
        start=time.perf_counter();session.run(None,{'input_features':features.numpy()});times.append(time.perf_counter()-start)
    report['onnx_seconds_per_10s']=times
    report['peak_rss_bytes']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
except Exception as error:
    report['export_error']=repr(error)
(ROOT/'runtime-report.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2),flush=True)
