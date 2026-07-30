/*
ラベル+1になるやつP個
ラベル-1になるやつQ個

positive feature N個, negative feature M個をそれぞれ決めて、
正例transactionにはpositive featureを確率pで追加、negative featureを確率rで混入
負例transactionにはnegative featureを確率qで追加、positive featureを確率rで混入

各transactionについて、追加するfeatureが決まったら(確率的に選択したら)
それらをつなぎ合わせる形で生成。
生成されたグラフの重複をminimum DFS codeでみる


該当実装↓↓

 // output the transactions

  for(int i=0; i<P; ++i){
    Graph out_g;
    for(int j=0; j<N; ++j){
      if(runif(0,1)>p) continue;
      //std::ostringstream os;
      //os << "pos_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_pos[j], num_elab);
    }
    for(int j=0; j<M; ++j){
      if(runif(0,1)>r) continue;
      //std::ostringstream os;
      //os << "pos_ctrl_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_neg[j], num_elab);
    }
    //print_graph(out_g);
    std::ostringstream os2;
    //os2 << "pos_" << i << ".sif";
    //output_sif(out_g,os2.str());
    os2.str("");
    os2 << "pos_" << i << "_edge_" << out_g.edgeline;
    output_gspan(out_g,ofs1,os2.str(),1);

    std::cout << "pos[" << i << "] edge=" << out_g.edgeline << std::endl;
  }

  for(int i=0; i<Q; ++i){
    Graph out_g;
    for(int j=0; j<M; ++j){
      if(runif(0,1)>q) continue;
      //std::ostringstream os;
      //os << "neg_comp_" << i << "-" << j << ".sif";
      //output_sif(g_neg[j],os.str());
      graph_append(out_g, g_neg[j], num_elab);
    }
    for(int j=0; j<N; ++j){
      if(runif(0,1)>s) continue;
      //std::ostringstream os;
      //os << "pos_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_pos[j], num_elab);
    }
    //print_graph(out_g);
    std::ostringstream os2;
    //os2 << "neg_" << i << ".sif";
    //output_sif(out_g,os2.str());
    os2.str("");
    os2 << "neg_" << i << "_edge_" << out_g.edgeline;
    output_gspan(out_g,ofs1,os2.str(),-1);

    std::cout << "neg[" << i << "] edge=" << out_g.edgeline << std::endl;
  }
*/
