"""Synthetic wire evidence only; no binaries or network execution."""
from copy import deepcopy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('workload_wires', Path(__file__).resolve().parents[1]/'netlab-workload-wires.py')
wires = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wires)


def fixture():
    client, server = [], []
    for i in range(424):
        op = 'directory' if i < 360 else 'fetch_path'
        offset = None if i < 360 else (i-360)*256*1024
        common = dict(operation=op, stream=str(i*4), offset=offset, alpn='khor/0')
        client.append(dict(pid=1, peer='y', phase='list_response_received' if i<360 else 'path_slice_received',
            details=dict(common, wire=10, early=False, elapsed_us=50000, timing_origin='exchange_started',payload_bytes=None if i<360 else 256*1024)))
        server.append(dict(pid=2, peer='x', phase='list_response_sent' if i<360 else 'path_slice_response_sent',
            details=dict(common, wire=20, timing_origin='stream_accepted', frame_bytes=1024,
                         permit_asked_us=0, seated_us=1, request_read_us=2, prepared_us=3, encoded_us=4, written_us=5)))
    client.append(dict(phase='tunnel_stream_ready',details=dict(wire=11,stream='0',alpn='khor-tunnel/0')))
    server.append(dict(phase='tunnel_target_accepted',details=dict(wire=21,stream='0',alpn='khor-tunnel/0',paired=True)))
    return client, server


class WireEvidence(unittest.TestCase):
    def test_actual_priority_readback_preserves_first_content_and_controls(self):
        for priority in [0, -1]:
            for mutation in ['success', 'first_slice_low', 'continued_wrong', 'directory_low', 'not_applied', 'missing_readback']:
                with self.subTest(priority=priority, mutation=mutation):
                    client, server = fixture()
                    for row in server[:-1]:
                        d = row['details']
                        d.update(priority_applied=True, response_priority=priority if d['operation']=='fetch_path' and d['offset']>0 else 0)
                    if mutation=='first_slice_low':server[360]['details']['response_priority']=-1
                    elif mutation=='continued_wrong':server[361]['details']['response_priority']=1
                    elif mutation=='directory_low':server[0]['details']['response_priority']=-1
                    elif mutation=='not_applied':server[361]['details']['priority_applied']=False
                    elif mutation=='missing_readback':del server[361]['details']['response_priority']
                    if mutation=='success':
                        result=wires.wire_evidence(client,server,continued_priority=priority)
                        self.assertEqual(result['matched_streams'],424)
                    else:
                        with self.assertRaises((AssertionError,KeyError)):
                            wires.wire_evidence(client,server,continued_priority=priority)

    def test_raw_business_identity_and_stage_facts_are_required(self):
        for mutation in ['success','missing_directory','missing_fetch','duplicate_offset','bad_stream','bad_Main_alpn',
                         'reused_stream','missing_tunnel','bad_tunnel_alpn','shared_tunnel_wire','unpaired','negative_clock','reversed_clock','wrong_bytes']:
            with self.subTest(mutation=mutation):
                client,server=deepcopy(fixture())
                if mutation=='missing_directory':client.pop(0)
                elif mutation=='missing_fetch':server.pop(360)
                elif mutation=='duplicate_offset':client[361]['details']['offset']=0
                elif mutation=='bad_stream':server[0]['details']['stream']='wrong'
                elif mutation=='bad_Main_alpn':client[0]['details']['alpn']='khor-tunnel/0'
                elif mutation=='reused_stream':server[1]['details']['stream']=server[0]['details']['stream']
                elif mutation=='missing_tunnel':client.pop()
                elif mutation=='bad_tunnel_alpn':client[-1]['details']['alpn']='khor/0'
                elif mutation=='shared_tunnel_wire':client[-1]['details']['wire']=10
                elif mutation=='unpaired':server[-1]['details']['paired']=False
                elif mutation=='negative_clock':server[0]['details']['seated_us']=-1
                elif mutation=='reversed_clock':server[0]['details']['prepared_us']=9
                elif mutation=='wrong_bytes':client[360]['details']['payload_bytes']=1
                if mutation=='success':
                    result=wires.wire_evidence(client,server)
                    self.assertEqual(result['matched_streams'],424)
                    self.assertTrue(result['all_business_on_one_Main'] and result['separate_Tunnel'])
                else:
                    with self.assertRaises(AssertionError):wires.wire_evidence(client,server)
