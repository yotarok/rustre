use automata::{StateMachine,Arc,FSA};
use automata::vector::{ByteVectorFSA};
use runner::{Runner};
use std::io::{Read};
use std::mem;
use std::ffi::{CString};
use std::env;
use std::ffi::OsStr;

#[allow(unused_imports)]
use utils::llvm::{
    create_module,Module,int8_ptr_type,int64_type,Builder,array_type,
    ExecutionEngine,add_function,ValueMethod,insert_basic_block,int_with_type,
    int64,int32,int8,int64_ptr_type,BranchTable,int8_type,int16_type,const_array,
    int32_type,insert_basic_block_after,TargetMachine,int128,int128_type,
    int128_ptr_type,int32_ptr_type,int16_ptr_type,int16,create_metadata_node};

use utils::rawbuffer::{RawBuffer};

use llvm_sys::{LLVMIntPredicate,LLVMLinkage};
use llvm_sys::prelude::{
    LLVMValueRef
};
use llvm_sys::core::LLVMMDString;
use llvm_sys::analysis::LLVMVerifierFailureAction;

use std::io::{stdout, Write, BufWriter};


pub struct CompileContext {
    module: Module,
    main_fn: LLVMValueRef,
}

impl CompileContext {
    pub fn new(module: Module, optfsa: ByteVectorFSA) -> CompileContext {
        let nstates = optfsa.nstates().expect("Number of states should be known");

        let mut module = module;
        let mut main_args = vec![
            int64_type(), // START STATE
            int64_type(), // BUFFER LENGTH
            int8_ptr_type(), // INPUT BUFFER
            int64_ptr_type(), // OUTPUT BUFFER
        ];
        let main_fn = add_function(&mut module, "run", &mut main_args, int64_type());

        let (arg_startst, arg_buflen, arg_inpbuf, arg_outbuf) = (
            main_fn.get_param(0), main_fn.get_param(1),
            main_fn.get_param(2), main_fn.get_param(3));

        arg_inpbuf.set_param_alignment(64);
        arg_outbuf.set_param_alignment(64);


        let state_type = if nstates < 0x100 {
            int8_type()
        } else if nstates < 0x10000 {
            int16_type()
        } else {
            int32_type()
        };
        let table_type = array_type(array_type(state_type, 256), nstates as u32);

        let table = {
            let mut table_vals = Vec::new();
            for st in 0..nstates {
                let mut row = Vec::new();
                row.extend_from_slice(&[0; 256]);
                for arc in optfsa.arcs(&(st as i64)) {
                    row[arc.label() as usize] = arc.nextstate() as u64;
                }
                let row_vals: Vec<_> =
                    row.iter().map(|s| int_with_type(state_type, *s)).collect();
                table_vals.push(const_array(state_type, &row_vals));
            }

            let table_initializer = const_array(array_type(state_type, 256), &table_vals);

            let table = module.add_global(table_type, c_str!("trans"));
            table.set_initializer(table_initializer);
            table.set_global_constant(true);
            table.set_linkage(LLVMLinkage::LLVMPrivateLinkage);
            table.set_alignment(64);
            table
        };

        let output_table = {
            let mut table_vals = Vec::new();
            for st in 0..nstates {
                let mut o = st as u64;
                if optfsa.final_weight(&(st as i64)) {
                    o |= 0x8000_0000_0000_0000;
                }
                table_vals.push(int64(o));
            }

            let table_initializer =
                const_array(array_type(int64_type(), nstates as u32), &table_vals);
            let table = module.add_global(array_type(int64_type(), nstates as u32),
                                          c_str!("output"));
            table.set_initializer(table_initializer);
            table.set_global_constant(true);
            table.set_linkage(LLVMLinkage::LLVMPrivateLinkage);
            table.set_alignment(64);
            table
        };

        let init_bb = main_fn.append_basic_block(c_str!("entry"));
        let loop_bb = main_fn.append_basic_block(c_str!("loop"));
        let final_bb = main_fn.append_basic_block(c_str!("exit"));

        let (unroll_count, _packed_type, packed_ptr_type, packed_const) = {
            //(1, int8_type(), int8_ptr_type(), |i| int8(i))
            //(2, int16_type(), int16_ptr_type(), |i| int16(i))
            //(4, int32_type(), int32_ptr_type(), |i| int32(i))
            (8, int64_type(), int64_ptr_type(), |i| int64(i))
            //(16, int128_type(), int128_ptr_type(), |i| int128(i))
        };

        // Build start routine
        let (inp_ptr_ptr, out_ptr_ptr, st_ptr, out_end_ptr) =  {
            let builder = Builder::new();
            builder.position_at_end(init_bb);

            let inp_ptr_ptr = builder.alloca(packed_ptr_type,
                                             c_str!("head_ptr_ptr"));
            let arg_inpbuf_casted = builder.pointer_cast(arg_inpbuf,
                                                         packed_ptr_type,
                                                         c_str!("head_ptr"));
            builder.store(arg_inpbuf_casted, inp_ptr_ptr);

            let out_ptr_ptr =
                builder.alloca(int64_ptr_type(), c_str!("out_ptr_ptr"));
            builder.store(arg_outbuf, out_ptr_ptr);

            let out_end_ptr = builder.gep(arg_outbuf, &vec![arg_buflen],
                                          c_str!("out_end"));
            out_end_ptr.set_in_bounds(true);

            let st_ptr = builder.alloca(state_type, c_str!("st_ptr"));
            let truncst = builder.trunc(arg_startst, state_type, c_str!("truncst"));
            builder.store(truncst, st_ptr);
            builder.br(loop_bb);

            (inp_ptr_ptr, out_ptr_ptr, st_ptr, out_end_ptr)
        };

        // main loop
        {
            let builder = Builder::new();
            builder.position_at_end(loop_bb);

            let mut curst = builder.load(st_ptr, c_str!("curst"));
            let mut curst_idx = builder.zext(curst, int64_type(), c_str!("curst"));

            let lab_packed_p = builder.load(inp_ptr_ptr, c_str!(""));
            let mut lab_packed = builder.load(lab_packed_p, c_str!(""));

            let mut out_p = builder.load(out_ptr_ptr, c_str!("out_p"));

            for _off in 0..unroll_count {
                let lab = builder.and(lab_packed, packed_const(0xFF), c_str!("lab"));
                let p = builder.gep(table, &vec![int64(0), curst_idx, lab], c_str!("nextst_p"));
                p.set_in_bounds(true);

                curst = builder.load(p, c_str!("curst"));
                curst_idx = builder.zext(curst, int64_type(), c_str!("curst"));

                let oval_p = builder.gep(output_table, &vec![int64(0), curst_idx],
                                         c_str!("val_p"));
                oval_p.set_in_bounds(true);
                let oval = builder.load(oval_p, c_str!("val"));

                builder.store(oval, out_p);
                out_p = builder.gep(out_p, &vec![int64(1)], c_str!("out_p"));
                out_p.set_in_bounds(true);

                lab_packed = builder.lshr(lab_packed, packed_const(8), c_str!(""));
            }

            builder.store(out_p, out_ptr_ptr);
            builder.store(curst, st_ptr);

            let lab_packed_p_next = builder.gep(lab_packed_p, &vec![int64(1)],
                                                c_str!("lab_packed_p"));
            lab_packed_p_next.set_in_bounds(true);
            builder.store(lab_packed_p_next, inp_ptr_ptr);

            let cmp = builder.icmp(
                LLVMIntPredicate::LLVMIntEQ,
                out_end_ptr, out_p, c_str!("reached"));

            let condbr = builder.condbr(cmp, final_bb, loop_bb);

            let children = vec![
                unsafe { LLVMMDString(c_str!("branch_weights"), 14) },
                int32(1),
                int32(256)
            ];
            let mdnode = create_metadata_node(children);

            condbr.set_metadata(2, mdnode);

        }

        // Build end routine
        {
            let builder = Builder::new();
            builder.position_at_end(final_bb);
            let st = builder.load(st_ptr, module.new_string_ptr("st"));
            let st = builder.zext(st, int64_type(), module.new_string_ptr("st"));
            builder.ret(st); // return the reached state
        }

        CompileContext {
            module: module,
            main_fn: main_fn,
        }
    }

    fn optimize_ir(&self, tm: &TargetMachine) {
        self.module.optimize(self.main_fn, tm);
    }

    #[allow(dead_code)]
    fn verify(&self) -> (bool, String) {
        self.module.verify(LLVMVerifierFailureAction::LLVMPrintMessageAction)
    }

    fn compile(self) -> (ExecutionEngine, extern "C" fn(u64, u64, *const u8, *mut u64) -> u64) {
        let ee = ExecutionEngine::new_mcjit(self.module);

        let addr = ee.get_function_addr(CString::new("run").unwrap().as_ptr());

        let f = unsafe {
            mem::transmute(addr)
        };
        (ee, f)
    }
}


// Output u64 design
// |is_final:1|blank:15|outsym:16|state:32|
pub struct JITFSARunner {
    #[allow(dead_code)]
    ee: ExecutionEngine, // need to own engine while func is being used
    func: extern "C" fn(u64, u64, *const u8, *mut u64) -> u64
}

impl JITFSARunner {
    pub fn new_with_optimized_fsa(optfsa: ByteVectorFSA) -> JITFSARunner {
        let (module, machine) = create_module("runner", None);

        let mut ctx = CompileContext::new(module, optfsa);

        ctx.verify();

        let empty = OsStr::new("").to_os_string();
        if env::var_os("RUSTRE_JIT_NOOPT").unwrap_or(empty.clone()).len() == 0 {
            ctx.optimize_ir(&machine);
        } else {
            eprintln!("[WARN] JIT optimization isn't applied");
        }

        if env::var_os("RUSTRE_JIT_DUMPIR").unwrap_or(empty.clone()).len() != 0 {
            ctx.module.dump();
        }



        match env::var_os("RUSTRE_JIT_DUMPASM") {
            Some(path) => {
                let path = path.to_str().expect("UTF-8 decode error (envvar)");
                machine.emit_module(&ctx.module, path).expect("Module emission error");
            },
            _ => {}
        }

        let (ee, f) = ctx.compile();

        JITFSARunner {
            ee: ee,
            func: f,
        }
    }
}

const BUFSIZE: usize = 64 * 1024;

impl<R: Read> Runner<R> for JITFSARunner {
    fn run(&mut self, input: R) {
        let out = stdout();
        let mut out = BufWriter::new(out.lock());

        let mut input = input;
        let mut inputbuf =  RawBuffer::new(BUFSIZE, 64);
        let mut outputbuf =  RawBuffer::new(BUFSIZE * mem::size_of::<u64>(), 64);

        let mut linebuf = Vec::<u8>::with_capacity(4 * 1024);
        let mut accepted = false; // this needs to kept for several chunks

        let mut prev_endstate = 0;
        let mut finished = false;
        while ! finished {
            let readsz = match input.read(inputbuf.as_slice_mut()) {
                Ok(sz) => {
                    if sz < inputbuf.size() {
                        finished = true;
                    }
                    sz
                },
                Err(_) => {
                    panic!("Read error")
                }
            };

            prev_endstate = (self.func)(prev_endstate,
                                        inputbuf.size() as u64,
                                        inputbuf.as_ptr::<u8>(),
                                        outputbuf.as_mut_ptr::<u64>());

            let mut linestart = 0;

            for (loc, (i, o)) in
                inputbuf.as_slice::<u8>().iter()
                .zip(outputbuf.as_slice::<u64>().iter())
                .enumerate()
            {
                if loc >= readsz {
                    break;
                }

                if *i == b'\n' {
                    if accepted {
                        linebuf.extend_from_slice(&inputbuf.as_slice::<u8>()[linestart..loc]);
                        out.write(linebuf.as_slice()).expect("Write error");
                        out.write(b"\n").expect("Write error");
                    }
                    // reset
                    linebuf.clear();
                    linestart = loc + 1;
                    accepted = false;
                }
                if (*o & 0x8000_0000_0000_0000) != 0 {
                    accepted = true;
                }
            }

            prev_endstate = prev_endstate & 0x7FFF_FFFF_FFFF_FFFF;
        }
    }
}

